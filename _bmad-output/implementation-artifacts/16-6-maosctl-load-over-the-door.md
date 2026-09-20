---
baseline_commit: "**`471608ea`** (Story 16-5 landed; working tree CLEAN). Authored 2026-09-18 from six scouts **and hardened by an independent validation round that disproved THREE of this story's own headlines (§15 V1-V3)** — a STATIC scout over the `maos run` admission block, a STATIC scout over 16-1's door seam, a STATIC scout over the `maosctl` surface, a STATIC scout over the kernel lifecycle table, a MEASURING scout that RAN `kloc-check`, `check-kernel-baseline`, `check-exit-commands`, `check-env-contract` and `check-workspace-count`, and a DOCUMENTARY scout over FR9, ADR-062 and the threat model. **Cites are SYMBOL-first, line second.** ⚠ **HEAD IS GREEN** — `kloc-check` PASSED (aggregate 167189, alarm firing, `over_budget: []`), `check-kernel-baseline` PASSED (24628 lines / 98 files), `check-exit-commands` PASSED (27 resolved, **4** owed — `eval`→18-2 and `maos-registry-server`/`maos-spirit`/`install`→20-1 — none to this key), `check-workspace-count` PASSED (55/55), `check-env-contract` PASS (94 vars, 0 violations). ⚠ **T0 re-measures everything. A grant is a global, not a reservation.**"
depends_on: "**`16-1-…` (`done`)** — this story is an extension of 16-1's seam, not a new surface. It needs `BinPrivateOps` (`crates/maos-bin/src/operator_door.rs:54`, impl `BinPrivateOpsImpl` `main.rs:8768`/`:8780`, constructed `main.rs:3330-3338`, handed to the door `main.rs:3339` (arg **18** of 19)), the `OperatorCommand` enum (`crates/maos-control/src/lib.rs:270`), the segment router `fn route` (`:967`, match `:982`), `submit_and_wait_with_deadline` + the `QUEUED → WITHDRAWN` CAS (`:1367`; comment `:1377-1381`, CAS `:1382-1390`), `SpiritCommandLocks` (`operator_door.rs:162`), the hand-rolled client `door_client::exchange` (`crates/maos-cli/src/door_client.rs:259`) and the D-16-1-P exit-code matrix (`:370-465`). **`16-3-…` (`done`)** — its `blocks:` front matter routes the pre-`start`/partial-load error-return residual here **by name**; 16-3 closed the two `--once` regions (`standalone_result` `main.rs:4982-5029`, `once_result` `:5848-5957`) and left the topology loop and the standalone load→admit→start run open. **`16-5-…` (`done`)** — it moved the kernel pin to **24628** (`xtask/kernel-core-baseline.toml:505`) and set `RATIFIED_AT_EASING = 19_053` (`xtask/tests/recovery_lane_ceiling_rule.rs:268`); both are artifacts this story must move again if D-16-6-A is ratified."
blocks: "**`epic-16`'s close.** With 16-0..16-5 all `done`, this is the last open Epic-16 story besides `epic-16-retrospective` (`sprint-status.yaml:241-253`). It also **closes one `epic-16-retrospective` row at origin** (rule 10(a)): the NFR-Rel-11 receipt gap for *a Spirit loaded but never started* (`sprint-status.yaml:253` cluster (a)) — this story is the first thing in the tree that deliberately produces that state, so it owns it."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT (rule 9, Round 10 — the 16-6 section, the stories-table row, the Kernel-Δ paragraph, the Kloc-asks paragraph, the Dependencies line, the exit block and its provenance list).** Measurement disproved premises in **all three** of the section's ACs. **AC1's admission order is INVERTED** — `security.admit_spirit` runs *after* `scheduler.load`, at the scheduler-assigned pid, on all three copies of the path (`main.rs:3800→3813` shell, `:4610→4729` topology, `:5181→5761` standalone), and the comments at `:4726-4727`/`:5757-5759` say why. **AC2 is FALSE at HEAD** — `is_transition_allowed` (`crates/maos-kernel-core/src/scheduler/control_block.rs:444-455`) has **no `(Loaded, Unloaded)` arm**, so a door-loaded Spirit cannot be unloaded at all; the SCB, its pid and its capability tokens leak for the daemon's life and the name is burned. The tree already documents this twice (`crates/maos-bin/src/supervision.rs:1339-1344`, `crates/maos-bin/src/shell_host.rs:242-247`). **AC3 is largely already discharged** — the residual-Worker threat is written in `README.md:282-284` and `docs/adr/ADR-062-mutating-operator-surface.md:161-165`. ⚠ **R4 (round-table 2026-09-18) — the precedent this sets, stated narrowly so it cannot rot: a story may CORRECT A MEASUREMENT; it may NOT RE-RULE A DECISION.** `kernel-Δ 0` is a measurement claim about the tree and it is false; D3-style rulings stay untouchable. ⚠ **The epic's `kernel-Δ 0` and its Dependencies note (*'16-6 is kernel-Δ 0, so it may land either side of 16-5 without a second re-pin'*) are both FALSE if D-16-6-A is ratified.** The rulings are the Decisions table; the applied edits are §12."
split_from: "Not a split. **Sizing: WHOLE.** One capability — *an operator adds a Spirit to a running daemon and can take it away again* — and the two halves (`load` the verb, `(Loaded, Unloaded)` the transition) are **the same fact**: the verb that creates the state owns the state's exit. Splitting them ships a leak as a feature. The rejected seam was `16-6b-loaded-unloaded-transition` at ◆ FLAG-Winston; it is refused because a `load` that cannot be undone is not FR9's `load`, and because the second story would be a one-line kernel edit whose only justification is the first story's defect — the shape 13.6e named (*a control whose owner has already shipped*). **Measured duration: 6–9 d.**"
kernel_grant: "**GRANTED by the operator 2026-09-19 (§17c; §14 Q1 CLOSED; §18a ruled (A)) — ◆ FLAG-Winston — and this contradicts the epic, deliberately (D-16-6-A, §1).** ⚠ **RE-SCOPED TWICE, RATIFIED WIDER, THEN PINNED BY ROUND 3: it is not one line, not one file, and the fourth file's edit is not one hunk.** Round 1 (§16a R3) added the operator posture ceiling; round 2 (§17 R10) proved the clamp needs **two** `min()`s, not one; the operator ruled §14 **Q4 = Shape A**, adding the fourth file; round 3 (§18 R21/R22) then proved **one of Q4's two offered mechanisms is broken** and pinned the other. The grant spans **FOUR kernel-core files**: (i) a `(Loaded, Unloaded)` arm in `fn is_transition_allowed` (`scheduler/control_block.rs:444-455`); (ii) an **`Option<Posture>` field named `operator_posture_ceiling`** — NOT `…_floor`; it is a `min()`, i.e. a ceiling, and a later 'fix' to `max()` bricks every Spirit at `Posture::Autonomous` via `shift_posture`'s `NonRuntimePosture` refusal (`capability/cap_policy/mod.rs:239-241`) — beside the existing floors in `OperatorPolicyConfig` (`capability/cap_policy/mod.rs:19-45`), legal values bounded to `{cautious, assistive, autonomous-with-halt}`; (iii) `min()` against it on **BOTH** `current` **and** `allowed_max` at `security/mod.rs:357-366` — clamping `allowed_max` alone stores `current > allowed_max` for orchestrator and butler (§17 R10); (iv) **three coordinated edits in `scheduler/scheduler_loop.rs`, not one** — (iv-a) `terminate_spirit`/`revoke_all_for_pid`/`drain_for_spirit`/`spirits.remove` move **AHEAD of `fire_on_unload`** in `SpiritSchedulerAdapter::unload` (`:471-523`), which is **mechanism (x); mechanism (y), demoting the `?` at `:502`, is REFUSED** because `check_hook_outcome`'s `Panicked` arm spawns `CrashDetector::handle_crash` and `terminate_spirit` is not idempotent, so the two teardowns race into duplicate receipts (§18 R21/R22); (iv-b) `check_hook_outcome`'s `Panicked` arm must **not** spawn `handle_crash` when `hook_name == \"on_unload\"` (`:573-595`) — the weaker 'only if the pid is still in the map' guard does NOT work, the pid still is at `:502`; (iv-c) `task.orphaned` + `enforce_disposition` are performed **inline** on the `on_unload`-failure path, because (iv-a) makes the spawned `handle_crash` return `NotLoaded` (`supervision/crash_detector.rs:78-84`) into a swallowed `let _ =` (`:581`). Estimated **`+16…+26`** kernel lines and now likely above it — **measure at T0 and again after `cargo fmt --all`, before asking for the raise.** ⚠ **`maos-kernel-core` headroom is ZERO, measured** (`kloc.toml:247`, comment ends *'ZERO HEADROOM retained'*), so the raise is REQUIRED, and `kloc-check` (`xtask/src/kloc_check.rs:220-388`) counts **tokei `code` lines** with `-e tests`, so `crates/*/tests/` is free while **inline `#[cfg(test)]` is charged**. ⚠ **`maos-kernel-core` is the SOLE member of `ZERO_HEADROOM_CRATES` (`xtask/tests/recovery_lane_ceiling_rule.rs:46`) and `authorize_raise` returns `Refusal::ZeroHeadroomCrate` (`:75-77`) BEFORE any citation check — the RECOVERY-LANE CEILING RULE does not authorize this at all.** ✅ But round 3 measured the escape: `authorize_raise` is **fixture-only** (all seven callers synthetic, `:128-204`) and never reads the real `kloc.toml`; the ONLY tree-bound assertion is `fn kernel_core_ceiling_has_not_moved_under_the_easing` (`:266-281`, `assert!(actual <= RATIFIED_AT_EASING)` at `:275`), and repo-wide `19053`/`19_053` occur in exactly TWO places: `kloc.toml:247` and `:268`. ⚠ **The re-pin is THIRTEEN EDITS ACROSS EIGHT ARTIFACTS — not the five 16-5 priced, not the seven an earlier draft claimed, not the ten §15 V5 corrected it to, and not the twelve round 2 counted before Q4 was answered:** (1) `xtask/kloc.toml:247` `maos-kernel-core = 19053` → N; (2) **`recovery_lane_ceiling_rule.rs:268` `RATIFIED_AT_EASING: i64 = 19_053`** — raising (1) without (2) reds `:275`; (3) `xtask/kernel-core-baseline.toml:505` `src_lines = 24628` → N, **which must stay on line 505** because `kernel_pin_content_hash_16_0.rs:842-851` asserts its line position; (4) the per-file SHA-256 for `scheduler/control_block.rs` (`:601`); (5) **`capability/cap_policy/mod.rs` (`:548`)**; (6) **`security/mod.rs` (`:614`)**; (7) **`scheduler/scheduler_loop.rs`** — locate its `[kernel_src.files]` row at T0; (8) the aggregate `set_hash` at `:540`; (9)–(12) **`kernel_pin_content_hash_16_0.rs` carries FOUR `24628` literals** — `:802`, `:820`, `:829`, `:840`; (13) a HISTORY row that **MUST be line-count-neutral** (every prior re-pin prepended a multi-line block; 16-5's is `:496-504`, nine lines — prepending again pushes `src_lines` off 505 and reds the position assert even with a correct value; §17 R15). **FOUR per-file digests change by name — AC5 must assert all four.** ✅ Round 3 verified the pin reconciles: **97 `.rs` files under `crates/maos-kernel-core/src` + `security/sandbox/t3-image.lock` = the pinned 98**, and `src_lines = 24628` is the **exact raw line sum** of those 97 files, so `--emit-pin` round-trips. ⚠ Plus the non-pin companion edits the \"eight artifacts\" counts: `scheduler/control_block.rs:567-573` `fn loaded_to_unloaded_rejected` (an existing unit test asserting the CURRENT refusal — inline `#[cfg(test)]`, so **charged**), `sprint-status.yaml:252`, the epic file and this file. [INFERENCE] a new public kernel field may also need a `xtask/kernel-api-classes.toml` class row (per-field precedent `RegionSection::home_region` `:572-574`, gate `check_service_boundary.rs:247-250`) — **verify at T0, do not assume.** Count them at T0 from the tree, never from this sentence. Re-pin command: `cargo run -p xtask -- check-kernel-baseline --emit-pin`."
kloc_grant: "MEASURED at `471608ea` by `cargo run -q -p xtask -- kloc-check --json` (PASSED, `alarm: true`, `over_budget: []`): **`maos-bin` 22996/22996 = ZERO** (`kloc.toml:477`) — ⚠ **the epic's `+250–450` is an ask against a crate with no headroom**; the Epic-16 lane allowance (`+480–820`, `:474`) was consumed and overdrawn by 16-1..16-5 (+1609, +281, +53, +5 booked at `:475-477`), and **`grep 16-6 xtask/kloc.toml` returns NO MATCH** — nothing is reserved. `maos-cli` 6080/6856 = **+776** (`:459`) — fits. `maos-control` 1264/1791 = **+527** (`:454`) — fits. `maos-journey-test` 626/1104 = +478 (`:634`). `maos-domain` 9268/9268 = **ZERO** (`:395`), `maos-shell` 606/606 = **ZERO** (`:644`), `maos-iac` 7084/7084 = **ZERO** (`:383`), `xtask` 44101/44101 = **ZERO** (`:320`, and behind the second ratchet `d11_xtask_ceiling_ratchet.rs`). **Aggregate 167189** against alarm 158608 (`:657`, **already FIRING**) and hardfail 170884 (`:658`) ⇒ **+3695 of room**; ⚠ the epic's R9 figure of 166542 / +4342 is one commit stale. Non-kernel crates may raise under the RECOVERY-LANE CEILING RULE (`kloc.toml:49-95`; lane OPEN — `epic-16` is `in-progress`, this key is `backlog`) provided the code exists, is `cargo fmt --all`-measured BEFORE the ask, and the figure + driver land in the same commit. ⚠ The tokenization rule is **not** in `kloc.toml` — it is `fn story_keys` in `xtask/tests/d11_xtask_ceiling_ratchet.rs:164-179`. ⚠ Inline `#[cfg(test)]` is charged; `tests/` is not."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` — it is a NEGATIVE marker: `check_dev_model_used_populated.rs:302` treats any line containing it as boilerplate and SKIPS it."
review: "§A6 full-layer net **BINDING and NON-DEGRADABLE**. The epic marks only 16-1 and 16-5 ◆, but this story acquires the same two properties that earned those marks: it performs a **kernel-core edit and moves the content-hash pin**, and it puts a **new mutating verb on the operator door**. ⚠ Round 2 adds a third: it ships a **privilege clamp** (§16a R3 / §17 R10), so the net includes a security layer by name. Blind + Edge Case + Acceptance + Test-Infra + Security + non-author runtime re-execution. E15-A6 binding by name — *does this test read the tree, or only what it set itself?* Obligations **(a)–(ai)** under **Review obligations** and §17d — (m2) is CORRECTED by §17 R16, (l) is PROMOTED to binding by §17 R11, and (g) is RE-AIMED by §17 R8."
---

# 16-6 — `maosctl load` over the door

Status: done

> **The capability:** *An operator with a running daemon can add a Spirit to it and take that Spirit away
> again. The Spirit is admitted through the same gates `maos run` applies — not a second, laxer copy —
> and it reaches the same states in the same order. `maosctl load` puts it in `Loaded`; `maosctl start`
> moves it to `Running`; `maosctl unload` removes it and writes the receipt that says it is gone. A load
> that is refused leaves nothing behind. A load that succeeds can always be undone.*

**Closes:** **FR9's `load` half** (`prd/functional-requirements.md:38` — ⚠ **the epic cites `:37`, which is FR8**) ·
the `epic-16-retrospective` NFR-Rel-11 row for *a Spirit loaded but never started*
(`sprint-status.yaml:253` cluster (a)) — **closed AT ORIGIN under rule 10(a)**, because this story is the first
thing in the tree that deliberately produces that state · the pre-`start`/partial-load error-return residual
routed here by **Story 16-3's `blocks:` front matter** (`16-3-…md:4`), which exists in **no `deferred-work.md`
row**.

**Does NOT close, and says so:**
- **FR9 as a whole.** `start`, `pause`, `resume` and `unload` were closed by 16-1. This story closes `load` —
  and, as a consequence nobody planned, it is what makes 16-1's **`start` reachable for the first time** (§3).
  FR9 is 5/5 only after this lands.
- **Third-party Spirit loading.** `classify_spirit` (`crates/maos-bin/src/main.rs:456`) is a fixed match over
  **eight compiled-in class names** (`enum LoadedSpiritKind` `:444-453`). `maosctl load` can only admit a class
  already linked into `maos-bin`. That is ADR-060's rule (`docs/adr/ADR-060-spirit-forms-by-trust-tier.md`:
  `rust-inproc` = compiled-in first-party only), not a shortcut — the third-party form is Epic 17's
  `wasm-component`. **AC1 states the bound rather than implying FR9 is wider than the tree.**
- **`NFR-Maint-9`** (`non-functional-requirements.md:140`, manifest schema N-1 compatibility). A `load` verb
  accepting an arbitrary operator manifest is exactly where N-1 compat becomes observable, and
  `admit_spirit` already has the window logic (`EAbiTooOld` `security/mod.rs:317`, `EAbiTooNew` `:323`,
  N-1 WARN `:332`). **The epic's ACs never mention it.** AC1's refusal vectors include it; the SLA itself is
  not claimed.
- **`FR51(a)/(d)`** — the bounded interrupt of an in-flight action. 16-1 raised it to the retrospective with
  no mechanism in the tree; nothing here changes that.
- **`requirements-inventory.md`'s FR9 homing.** `:437` says E1a+E5; `:547` homes all of
  `FR9/FR13/FR16/FR24/FR51` to `16-1`. Both predate the 2026-09-13 FR9 split and are repaired in §12 —
  but the inventory is a third source of truth and this story does not adopt it as one.

---

## What this story actually is

The epic files this as *"reuse `maos run`'s admission path behind 16-1's door."* Measured against the tree,
that sentence contains three errors and hides the story's real subject.

The real subject is **a state the kernel has no exit from**. Every Spirit that exists at HEAD was loaded and
started in the same breath by `maos run` (`main.rs:5181` → `:5761` → `:5823`), so the `Loaded` state is a
sub-second transient nobody can observe. `maosctl load` makes it a **durable, operator-visible state** — and
the moment it does, six latent defects become reachable:

1. **`Loaded` has no exit** (§1) — `is_transition_allowed` has `(Loaded, Running)` but not
   `(Loaded, Unloaded)`. The verb creates a Spirit that cannot be removed.
2. **Admission runs after load, not before** (§2) — the epic's AC1 lists the steps in the wrong order, and
   the real order is why a refused admission already leaks today.
3. **`maosctl start` has no operator-creatable subject** (§3) — 16-1 shipped a verb no operator has ever been able to
   succeed. This story is its first subject.
4. **The route shape does not exist, and its serialization key cannot** (§4) — every mutating route is
   `/v1/spirits/{id}/{verb}`, keyed by an id that a `load` does not have yet.
5. **The class set is eight compiled-in names** (§5) — "load a manifest" is not what the tree can do.
6. **The gate that should bind this story is vacuous** (§6) — `check-exit-commands` owes `16-6` nothing,
   because the epic's hermetic exit block contains no `load` token.

---

## 1. 🔴 THE VERB CREATES A STATE THE KERNEL CANNOT LEAVE — AC2 IS FALSE AND `kernel-Δ 0` IS FALSE

`fn is_transition_allowed` — `crates/maos-kernel-core/src/scheduler/control_block.rs:444-455`:

```rust
matches!(
    (from, to),
    (Loaded, Running)            // :448
        | (Running, Paused)      // :449
        | (Paused, Running)      // :450
        | (Running, Unloaded)    // :451
        | (Paused, Unloaded)     // :452
        | (Unloaded, Unloaded)   // :453  idempotent unload
)
```

**There is no `(Loaded, Unloaded)` arm.** A freshly loaded SCB is `Loaded`
(`control_block.rs:337`, `state: AtomicU8::new(ScbLifecycleState::Loaded as u8)`).

The epic's AC2 states verbatim (`epic-16-…md:193`): *"a door-loaded Spirit unloads through 16-1's `unload`."*
Traced through `SpiritSchedulerAdapter::unload` (`scheduler_loop.rs:472`) for a `Loaded` SCB:

| line | what happens |
|---|---|
| `:473-476` | `get_scb_optional` → `Some(scb)` — it IS in the map |
| `:478-481` | `current == Unloaded`? No, it is `Loaded`. No early `Ok` |
| `:483-486` | `scb.transition(Unloaded, Unload)?` → `control_block.rs:469-475` → **`Err(InvalidStateTransition)`. The `?` returns HERE.** |
| `:488-499` | ❌ never runs — no `lifecycle.unload` TL row |
| `:501-502` | ❌ `on_unload` never fires |
| `:504-512` | ❌ **`terminate_spirit` unreached — NO NFR-Rel-11 RECEIPT** |
| `:514` | ❌ `capability.revoke_all_for_pid` — **tokens stay live** |
| `:516` | ❌ `halt_registry.drain_for_spirit` — pending halts stay pending |
| `:518-521` | ❌ `spirits.remove(&pid)` — **the SCB LEAKS PERMANENTLY** |

Over the door this is `409 invalid_state_transition` (`operator_door.rs:479-488`).

**Second-order: the name is burned.** Because `spirits.remove` never runs, `resolve_pid` (`scheduler_loop.rs:178-186`,
a linear scan over `scb.spirit_id`) keeps matching forever, so every later `load` of that id returns
`AlreadyLoaded` (`:268-272`).

⚠ **Scope, precisely: this is true of `SpiritSchedulerAdapter::unload`, NOT of the tree.** An earlier draft of
this section claimed *"no force-unload, no purge, no admin escape hatch anywhere in the tree"* — **that was
false and is withdrawn.** Two production paths already remove a `Loaded` SCB, both by bypassing `transition`:

| path | how it bypasses | what the Spirit gets |
|---|---|---|
| `handle_crash` (`crates/maos-kernel-core/src/supervision/crash_detector.rs:120-124`) | `let _ = scb.transition(Unloaded, Unload);` — the `Err` is **discarded** and execution continues | tokens revoked `:127`, **`terminate_spirit` receipt `:130`**, `LifecycleEvent::Crash` journal `:181-189`, and `map.remove(&spirit_pid)` **`:191-194`** |
| cold-swap rollback (`crates/maos-kernel-core/src/lifecycle/upgrade.rs:200-203`) | direct `scbs().write().remove(&new_pid)`, no `transition` call at all | nothing — no row, no receipt, no revocation, no drain |

And `SpiritSchedulerAdapter::scbs()` (`scheduler_loop.rs:148`) is **`pub`**, handing out
`Arc<RwLock<BTreeMap<u32, Arc<SCB>>>>` — so any `maos-bin` caller could remove an entry in three lines at
**zero kernel-Δ**. That is a real alternative to the grant, and **D-16-6-A now argues against it explicitly**
rather than pretending it does not exist.

⚠ **AND AN EXISTING TEST PINS THE CURRENT BEHAVIOUR.** An earlier draft claimed *"zero test coverage of the
exact case … the falsifier AC2 needs has never existed"* — **false on both halves.**
`crates/maos-kernel-core/src/scheduler/control_block.rs:567-573`:

```rust
#[test]
fn loaded_to_unloaded_rejected() {
    assert!(!SpiritControlBlock::is_transition_allowed(
        ScbLifecycleState::Loaded, ScbLifecycleState::Unloaded));
}
```

Adding the arm **reds `cargo test -p maos-kernel-core`**. This test is an **eighth re-pin artifact**, it must be
rewritten in the shape of its five siblings (`loaded_to_running_allowed` `:515-525`), and because it is inline
`#[cfg(test)]` in `maos-kernel-core` it **is charged by `kloc-check`** — so §10's kernel row is `+3…+6`, not
`+1`.

**The tree already knows.** Two production comments name this defect:
- `crates/maos-bin/src/supervision.rs:1339-1344` — *"The SCB is `Loaded` and there is no `Loaded → Unloaded` transition, so it cannot be unloaded and gets no receipt — the declared residual on the `epic-16-retrospective` row."*
- `crates/maos-bin/src/shell_host.rs:242-247` — *"…so when `load` succeeded but `admit_spirit` did not, the SCB is still `Loaded` and this unload ALWAYS fails."*

And `unload_all_loaded` (`supervision.rs:139-167`) swallows it: a `Loaded` SCB lands in `report.failed` `:157`
and is skipped with `continue`.

⇒ **There is no zero-delta option that satisfies AC2 as written.** The three shapes are ruled in
**D-16-6-A**; the recommendation is the one-line kernel arm, and its true price is **seven artifacts**
(frontmatter `kernel_grant`), not the five 16-5 priced — because `RATIFIED_AT_EASING`
(`xtask/tests/recovery_lane_ceiling_rule.rs:268`) and the pin's **line-position assert**
(`kernel_pin_content_hash_16_0.rs:846-851`) are two nobody has counted before.

⚠ **Integration coverage, separately.** `crates/maos-kernel-core/tests/scheduler_five_verb_lifecycle.rs` always
`start`s first (`:143`, `:182`); `invalid_state_transition_rejected` (`:190-209`) tests **`pause` on `Loaded`**,
never `unload`. So the *behavioural* falsifier AC2 needs does not exist — but the *unit* assertion above does,
and it asserts the opposite of AC2.

---

## 2. 🔴 ADMISSION RUNS *AFTER* LOAD — THE EPIC'S AC1 ORDER IS INVERTED

The epic's AC1 (`epic-16-…md:192`) lists: *"manifest parse, `security.admit_spirit`, the rust-inproc class
construction, `SpiritSchedulerAdapter::load`."* **Measured, the order is the reverse of the middle two**, on
all three copies of the path:

| path | load | admit | start |
|---|---|---|---|
| shell | `main.rs:3800` | `:3813` | `:3855` |
| run / topology | `:4610-4725` | `:4729` | `:4769` |
| run / standalone | `:5181-5756` | `:5761` | `:5823` |

The tree states why, at `main.rs:4726-4727` and `:5757-5759`: **capability mediation is keyed by pid**, so
admitting a placeholder pid would make every live port fail closed. The pid does not exist until
`allocate_pid()` inside `load` (`scheduler_loop.rs:274`).

**Consequence for AC1.** An "extracted admission function" cannot be admission-then-load. It must own the
whole **load → admit → start** triple, *and* own the rollback that today's `?` at `main.rs:5788` does not do:
an `admit_spirit` rejection leaves the Spirit **loaded in the scheduler, unadmitted**, and exits `main` — and
by §1 that Spirit cannot be unloaded. **The partial-admission leak and the missing transition are the same
defect seen from two ends.**

⚠ **The kernel's own `load` admission is not the gate.** `SpiritSchedulerAdapter::load` calls
`security.admit_spirit` itself (`scheduler_loop.rs:277-323`) — but with **hardcoded** `SandboxTier::T2`, no
image pin, all-`None` `ResourceCaps`, **empty** `CapabilitiesRequired` and a fixed
`Cautious`/`AutonomousWithHalt` posture. It threads only `manifest.scheduling`, `.lifecycle`, `.on_crash`,
`.supervision` and `.class`. **A door `load` that called the kernel directly would admit a T3-requiring Spirit
at T2** — a sandbox downgrade. Every real gate lives in `maos-bin`, which is precisely why AC1 must reuse the
extracted function and not the kernel entry point.

### 2b. The gates `maos run` actually applies — re-derived, because AC1 owes one refusal vector per gate

**In `main.rs`, standalone path (the stricter of the two — see 2c):**

| # | gate | cite |
|---|---|---|
| 1 | manifest readable | `main.rs:4153-4158` |
| 2 | manifest TOML parses | `:4159-4160` |
| 3 | `[cli_wrapper]` ⊕ `[class]` mutual exclusion | `:4962-4969` |
| 4 | `[class]` present + parses | `:5083-5084` |
| 5 | **known class name** (`classify_spirit`) | `:5085-5091` |
| 6 | `[sandbox]`/`[resources]`/`[output_shape]`/`[posture]` parse | `:5092-5101` |
| 7 | `[epistemic_policy]`/`[scheduling]`/`[lifecycle]`/`[budget]` parse | `:5102-5125` |
| 8 | **model-provenance admission** (`maos_registry::admission::validate_model_provenance`) + FR62 TL event | `:5135-5143` |
| 9 | butler boot-loud self-halting-posture guard | `:5206-5214` |
| 10 | mira boot-loud scalar-port guard | `:5700-5706` |
| 11 | **`security.admit_spirit`** | `:5760-5786` |

**Inside `admit_spirit` (`crates/maos-kernel-core/src/security/mod.rs:245`):**

| gate | line |
|---|---|
| `EClassRequired` | `:278-280` |
| `ESubstrateTooOld` (unparseable / kernel < `min_substrate_version`) | `:295`, `:301` |
| `EAbiTooOld` / `EAbiTooNew` (manifest schema window) | `:317`, `:323` |
| N-1 WARN degradation (non-fatal) | `:332` |
| **T3 image-lock load + verify** → `T3AdmissionFailed` | `:385-402` |
| T3 `image_pin` resolve / `default_entry` | `:403-408` |
| `SandboxTierUnsupported` (T4) | `:410-412` |
| `SandboxTierUnsupported` (T1, fail-closed) | `:414-417` |
| resource-cap strictest-of (manifest vs operator floor) | `:418-426` |

⚠ **DISPROVED — there is NO trust-tier, signature, region-pin or vetting gate in the `maos run` path.**
A case-insensitive scan of `main.rs:4094-6047` for `trust_tier|signature|verify|region|vetted|attestation|keyring`
returns **zero** hits. `trust_tier` inside `admit_spirit` is *read from the policy table with a
`TrustTier::Verified` default* (`security/mod.rs:338-343`, `:372-377`) — an input to `effective_sandbox_tier`,
not a refusal. The real signature/region/vetting gating lives in a **different** `admit_spirit`
(`crates/maos-registry/src/admission.rs:179`) that `maos run` never calls. **AC1 must not claim gates the path
does not have.**

⚠ `enforce_vetted_upgrade_precondition` is **`main.rs:134`** (the epic's `:6452,:6734` are the `smoke-epic-4`
distillation arm and `PanicSpirit` respectively). Its only callers are the `BinPrivateOps` impl
(`main.rs:8840`) and, through it, `operator_door.rs:709` and `:799`. It is **not** a `maos run` gate — the
epic's AC1 says so and is right, but for the wrong reason.

### 2c. The two run paths disagree, and the extraction must pick one

- **standalone** runs model-provenance **before** `load` (`:5135`) and emits `emit_vetter_key_event` on **both**
  grant and rejection (`:5789` / `:5779`).
- **topology** runs model-provenance **after** `admit_spirit` (`:4757`) and emits the vetter event on **grant
  only** (`:4749`).

D-16-6-C picks the standalone order (refuse before allocating a pid). The divergence is a real behavioural
difference between two shipped paths. ⚠ **SUPERSEDED (operator, 2026-09-19 — §14 Q5 = Shape A, §17c): the
divergence is RESOLVED, not recorded.** AC1 re-points **both** file-manifest arms at the one extracted
function, so both adopt the standalone order and topology's `emit_vetter_key_event` gains its **rejection**
row (the `map_err` at `:4746-4748`, today silent). §11 row 2 is no longer a cut line. Shell stays out of scope
by name — it parses a compiled-in manifest and is not a third copy of this path.

---

## 3. 🔴 `maosctl start` HAS NO OPERATOR-CREATABLE SUBJECT AT HEAD — THIS STORY IS ITS FIRST

`lifecycle_command` (`crates/maos-bin/src/operator_door.rs:430`) gates `start` on the SCB state
(`:449-470`, **D-16-1-J**):

```rust
if verb == "start" {
    let current = self.scb_state(spirit_id);
    if current != Some(ScbLifecycleState::Loaded) {
        return OperatorOutcome::Conflict { code: "invalid_state_transition", … };
    }
}
```

`start` succeeds **only** from `Loaded`. But every root loads, admits and starts in one unbroken run
(`main.rs:5181` → `:5761` → `:5823`; topology `:4610` → `:4729` → `:4769`) **before** the serving loop the
door answers from. So at HEAD, by the time any `maosctl` command can reach the daemon, **no Spirit is ever in
`Loaded`** — and `maosctl start <spirit>` can only ever return 409.

16-1 shipped a verb with no operator-creatable subject. **`maosctl load` is what gives it one.** This is the strongest argument
for D-16-6-A Shape B: a `load` that auto-starts (Shape A) would leave `start` unreachable forever and leave
FR9 at 4/5 verbs while claiming 5/5.

⚠ **Precision — the two places a `Loaded` SCB does briefly exist, and why neither is a subject.** (a) Cold-swap
inserts a `Loaded` successor SCB into the map (`lifecycle/upgrade.rs:194-198`) and calls `start` on the very
next statement (`:199`); the window is a few instructions inside one `async fn`, it is not addressable by
`spirit_id` (the predecessor still holds the name), and cold-swap is **refused over the door** anyway
(`operator_door.rs:632-643`). (b) A failed `admit_spirit` (`main.rs:5788`) or a failed `on_load`
(`scheduler_loop.rs:368`, after the insert at `:348`) strands a `Loaded` SCB — but both return `Err` out of
`main`, so the process is exiting and no door answers.

⚠ **CORRECTED (validation round). A THIRD producer exists, it is durable, and it is inside a SERVING root — so
the bare claim *"no `Loaded` Spirit exists while a door serves"* is FALSE and is withdrawn.** The door is
constructed at `main.rs:3339` and **bound and serving at `:3362`, before the run block at `:4094`.** The Worker
path then runs inside that serving root: `worker_spawn.rs:697` → `supervision.rs:1334` `scheduler.load(...)` →
`:1339` `scheduler.start(pid)`. A failed `start` returns `Err` and **leaves the SCB durably `Loaded` in the
map**, under an id that passes `validate_id` (`worker` / `worker-N`, `supervision.rs:1289-1293`) — exactly the
residual its own comment declares at `:1340-1342`. Even the happy path leaves a `Loaded` window between the two.

**The true claim, and the one AC3 asserts:** there is **no OPERATOR-INITIATED `Loaded` subject** at HEAD. Every
root loads+admits+starts in one unbroken run before serving, and the only durable HEAD producer is a **failed
Worker `start`** — a fault an operator cannot cause on purpose, schedule, or rely on. `maosctl start` has
therefore never had a subject an operator could deliberately create, and `load` is what gives it one.

⚠ AC1 therefore asserts the **three-step operator sequence**, not a single verb:
`maosctl load <manifest>` → `lifecycle_state: Loaded` → `maosctl start <id>` → `Running` → `maosctl unload <id>`
→ gone, with a receipt.

---

## 4. 🔴 THE ROUTE SHAPE DOES NOT EXIST, AND THE SERIALIZATION KEY CANNOT BE COMPUTED

**There is no collection route.** `fn route` (`crates/maos-control/src/lib.rs:967`, match `segments.as_slice()` `:980`) matches by
path segments. The spirit arms are `["v1","spirits",id,"sandbox"]` (`:1001`), `["v1","spirits",id]` GET-only
(`:1014`) and `["v1","spirits",id,verb]` POST (`:1036`). **`["v1","spirits"]` is not an arm** — it falls to
`_ => 404` (`:1211`). So a new arm is clean, with no ordering conflict against the delicate
`{id}` / `{id}/sandbox` disambiguation the doc comment at `:963-966` warns about.

**The `{id}/{verb}` route cannot carry a load.** Three independent reasons:
1. `validate_id` (`:1443`) permits only `[A-Za-z0-9._-]{1,128}` — **a filesystem path can never be a path
   segment**.
2. `lifecycle_command` returns `spirit_not_loaded` (404) from `resolve_pid` **before doing anything**
   (`operator_door.rs:438-440`) — the guard is exactly inverted for a load.
3. The id does not exist until the manifest is parsed.

**The serialization key cannot be computed before the work.** `OperatorCommand::spirit_key()`
(`maos-control/src/lib.rs:345-364`) is a **total function** and is the D-16-1-R per-Spirit lock key
(`operator_door.rs:1311` `run_command`, lock registry `SpiritCommandLocks` `:162`). A
`Load { manifest }` has no `spirit_id` to return, so it must return `None` — i.e. **it is not serialized**,
exactly like `ForgetMemory` and `ReleaseLegalHold`, the two commands whose unserialized race 16-5 spent AC3
fixing.

**And the kernel's own duplicate guard is not atomic either.** In `load`, `resolve_pid` (`scheduler_loop.rs:268`)
and `spirits.insert` (`:348`) are **two separate `RwLock` acquisitions** separated by `allocate_pid()` and the
entire `admit_spirit` call. Two concurrent loads of the same id both observe `None` and both insert under
different pids. This is masked **only when** `revocation_rules` is `Some` — `admission_guard()`
(`crates/maos-kernel-core/src/revocation/rules.rs:33-35`) is a `tokio::sync::Mutex` held from `:256` to
end-of-`load`. **When it is `None` there is no serialization at all**, and AC2's "typed `409 AlreadyLoaded`"
is a check-then-act.

⇒ **D-16-6-D**: the door handler parses the manifest first, learns the id, and takes
`self.spirit_lock(&spirit_id)` **itself** before calling the extracted function — a correct critical section
even though `spirit_key()` is `None`. `route_budget()` (`:367-375`) gets a `Load` arm returning
`LONG_ROUTE_BUDGET` (30 s, `:227`) beside `Upgrade`/`Uninstall`/`ImportRevocations`/`HotSwapPrecheck`, because
admission does T3 image-lock verification.

⚠ **A new variant lands on TWO surfaces.** `crates/maos-bin/src/shell_host.rs:20,51,171` holds an
`Arc<dyn OperatorCommandPort>` and calls the same `submit_and_wait`. Whatever `load` does over HTTP, the shell
REPL path sees.

⚠ **The manifest path is the operator's, the daemon's CWD is not.** D-16-1-X (`maos-control/src/lib.rs:322-324`)
already ruled this — it is why `ImportRevocations` carries `crl: Vec<u8>` bytes, not a path. The CLI must
canonicalise before sending, using the existing `fn canonical_manifest` (`crates/maos-cli/src/subcommands.rs:93-103`,
missing ⇒ exit 1, non-canonicalisable ⇒ exit 1) exactly as `spirit upgrade` does (`:2291`). ⚠ Note that
`maos run` itself does **not** canonicalise (`main.rs:4152` `PathBuf::from` + `read_to_string` in its own CWD) —
so AC2's canonicalisation is **new behaviour, not HEAD behaviour**, and the extracted function must accept an
already-canonical path from both callers.

---

## 5. 🔴 THE CLASS SET IS EIGHT COMPILED-IN NAMES — "LOAD A MANIFEST" OVERSTATES THE TREE

`fn classify_spirit` (`main.rs:456`, arms `:458-465`, `None` `:466`) is a fixed string match returning `enum LoadedSpiritKind`
(`:444-453`): `Butler, Researcher, Orchestrator, Architect, Reviewer, Mira, Nash, Digest`. `None` otherwise.

- **`rust-inproc` is not a class** — it is a `[class].forms` string.
- **`wasm` is not a class** at all.
- **`[cli_wrapper]` never reaches `classify_spirit`** — it is forked on first (topology `:4238`,
  standalone `:4962`).
- `SpiritSchedulerAdapter::load` is **generic over a compile-time Rust type**
  (`load<T: Spirit + Send + Sync + 'static>`, `scheduler_loop.rs:243`). A manifest cannot name a type; the
  class dispatch is a hand-written `match` monomorphising per arm.

⇒ **A door `load` can only ever admit a class already linked into `maos-bin`.** AC1 says so. The refusal
vector for an unknown class is gate #5 (`main.rs:5085-5091`).

⚠ **The construction switch already exists in a fourth place — but it is INCOMPLETE, and that is a trap.**
`successor_factory` (`main.rs:3123-3198`, the `class.name.as_str()` match at `:3130`) produces
`Arc<dyn AnySpiritObj>`, already `'static` and Arc-captured (`:3110-3122`). It is the class-dispatch half of
the extraction, pre-built — **but it covers SEVEN of the eight classes plus `"smoke-spirit"`, and has NO
`"researcher"` arm** (arms: `orchestrator` `:3131`, `architect` `:3134`, `reviewer` `:3137`, `mira` `:3141`,
`nash` `:3144`, `digest` `:3147`, `smoke-spirit` `:3150`, `butler` `:3163`, then
`unsupported => UpgradeError::SuccessorFactory` `:3186`). Meanwhile `classify_spirit` DOES map `"researcher"`
(`main.rs:459`) and the standalone arm loads one (`:5645`). **Building the extraction naively on
`successor_factory` would silently make `maosctl load` unable to admit a class `maos run` can** — a capability
regression disguised as reuse. Extend it with the missing arm, or dispatch classes separately; do not assume it
is complete, and do not write a fifth copy.

⚠ **`scheduler.load(...)` appears 20 times in `main.rs`**, not "a dozen": shell `:3801`; topology `:4612, :4623,
:4635, :4682, :4698, :4709`; standalone `:5363, :5645, :5669, :5678, :5688, :5725, :5739, :5748`; one-shot smoke
arms `:6655, :6742, :6777, :6830, :6985`. The extraction targets the **three real copies** (shell, topology,
standalone) and leaves the five smoke arms alone.

---

## 6. 🔴 THE GATE THAT SHOULD BIND THIS STORY IS VACUOUS

`cargo run -q -p xtask -- check-exit-commands --json` at HEAD:

```json
{"passed":true,"epics_in_roster":6,"blocks_read":6,"tokens_resolved":27,
 "owed":[{"token":"eval","owner_story":"18-2-…"},
         {"token":"maos-registry-server","owner_story":"20-1-…"},
         {"token":"maos-spirit","owner_story":"20-1-…"},
         {"token":"install","owner_story":"20-1-…"}],
 "findings":[],"surfaces":{"maos":8,"maosctl":25,"xtask":95,"one_shot_modes":32}}
```

(Abridged — the real payload carries `epic`/`owner_status`/`claimed_in` on every `owed` entry plus a
`surface_notes` array. **Four** owed at HEAD, not three; an earlier draft omitted `maos-spirit`.)

**It owes `16-6` nothing** — and it never will, because **epic-16's hermetic exit block
(`epic-16-…md:27-35`) contains no `load` token.** `grep "load"` over those nine lines returns zero matches.

Epic-16's **confidence rule 1** is *"verb-first (exit verbs exist at HEAD or are AC1 in this epic;
`check-exit-commands`)"* (`:57`). For this story that rule is **vacuous as the block stands**: the epic's own
hermetic exit does not exercise the capability 16-6 ships, and the gate stays green no matter what happens
here.

⇒ **D-16-6-F: add exit line 5b, and its provenance bullet, in the creation commit.** (Positioned after line 5's
root and **before** line 6's `kill "$RUN_PID"` — a line numbered 8 after line 7's `maos purge` would have had
no daemon to talk to, and renumbering would have invalidated every provenance bullet.)

✅ **DONE AND MEASURED IN THE CREATION COMMIT.** With the line and its bullet in place and the binaries built,
`./target/debug/xtask check-exit-commands --json` reports:

```json
{"passed":true,"tokens_resolved":31,"findings":[],
 "owed":[{"token":"load","epic":16,"owner_story":"16-6-maosctl-load-over-the-door",
          "owner_status":"ready-for-dev","claimed_in":"story file 16-6-…md"}, …]}
```

Resolved tokens **27 → 31** (the line's `spirit`, `start`, `unload`, `inspect` all resolve against real
`maosctl --help`), zero findings, and `load` is now **owed to this key by name** — so rule 1 binds, and the
gate will flip `load` from `owed` to `resolved` the moment the verb ships. `bash -n` over the whole 8-line
block is clean, and the line carries no `<placeholder>` so `PlaceholderHazard`
(`check_exit_commands.rs:849-877`) is not in play.
⚠ **The `maos`/`maosctl` binaries must be BUILT before running this gate** — it resolves verbs from live
`--help`, so an unbuilt tree reports `binary 'maos' does not exist at HEAD` and 12 spurious `UnresolvedVerb`
findings across four epics. That is a tooling artifact, not a red.
⚠ This is the **second** exit-block COMMAND edit in the epic's history (16-4's `--yes` was the first). It
requires a clean `bash -n` dry-parse, and any `<placeholder>` must be **quoted** or `PlaceholderHazard`
(`check_exit_commands.rs:849-877`) reds it — the F15 defect that already hit this block's line 3.

⚠ **And do not copy 16-4's CI precedent.** `grep -n "maos_uninstall_16_4\|purge" .github/workflows/discipline.yml`
returns **nothing**: 16-4's test has no named job and runs only inside the 45-minute `workspace-test-suite`
(`:3559`) — the exact "one red line in a 45-minute run" failure the `one-daemon-one-door` comment
(`:3640-3642`) exists to prevent, and it contradicts `epic-16-…md:25`, which asserts exit lines 5–7 run under
their own leg. 16-6 gets a **named job** (AC6).

---

## 10. SIZING AND BUDGET (rule 6, ×1.3; interval arithmetic)

| crate | headroom @ HEAD | change | raw | ×1.3 upper |
|---|---|---|---|---|
| `maos-kernel-core` | **0 — FLAG-Winston only** | one `(Loaded, Unloaded)` arm in `is_transition_allowed` (`control_block.rs:448-453`) **+1**, PLUS rewriting `fn loaded_to_unloaded_rejected` (`:567-573`) in the shape of its five siblings (`loaded_to_running_allowed` `:515-525`) — inline `#[cfg(test)]`, **which `kloc-check` charges** | **+3…+6** | **+8** |
| `maos-bin` | **0 — measured raise required** | the extracted `load → admit → start` function + its rollback (new module `admission.rs`), the `Load` handler in `operator_door.rs`, the `BinPrivateOps::load_spirit` method + `main.rs:8780` impl, **minus** the three collapsed copies (shell `:3740-3930`, topology `:4600-4792`, standalone `:5083-5828`) | **+180…+420** | **+546 (measured raise)** |
| `maos-control` | +527 | `OperatorCommand::Load` variant + `spirit_key`/`route_budget` arms + the `["v1","spirits"]` POST arm + body parse | +45…+90 | **+117** |
| `maos-cli` | +776 | `Load(LoadArgs)` clap variant + dispatch + `canonical_manifest` reuse + D-16-1-P mapping | +40…+85 | **+111** |
| `maos-journey-test` | +478 | exit-line-8 assertions if AC6's scene leg lands there | +0…+40 | **+52** |
| `maos-domain` | **0** | ⚠ **none intended.** If a typed refusal leaks into `maos-domain` — which is what happened to 16-5 (`kloc.toml:395`) — it is a **second uncosted raise**. Reuse `OperatorOutcome` (`maos-control/src/lib.rs:384-399`); do not mint a domain error | 0 | 0 |
| `xtask` | **0** | pin/ceiling artifacts only (TOML + `tests/`), which `kloc-check` does not charge | 0 | 0 |

**Two crates cross:** `maos-kernel-core` (FLAG-Winston only — the easing explicitly does **not** apply) and
`maos-bin` (measured raise under the RECOVERY-LANE CEILING RULE). **Code first, `cargo fmt --all`,
`kloc-check --json`, then the `kloc.toml:477` row carries the bare token `16-6` followed by a non-digit
non-dash character, the measured figure and the driver, same commit.** ⚠ The tokenization rule is enforced by
`fn story_keys` in `xtask/tests/d11_xtask_ceiling_ratchet.rs:164-179`, **not** by `kloc.toml:49-95` where a dev
will look for it.

⚠ **CORRECTED (validation round): they are NOT three verbatim copies, and an earlier draft biased this forecast
the wrong way against a ZERO-headroom crate.** Only **topology** (`:4610→4729→4769`) and **standalone**
(`:5181→5761→5823`) share a shape, and even they diverge — §2c records the provenance ordering AND the
vetter-event asymmetry. The **shell** triple (`:3800→3813→3855`) is a different animal: it parses a
**compiled-in** manifest string (`maos_spirit_hello::MANIFEST_TOML`, `main.rs:3741`) with **no file read, no
`classify_spirit`, no model-provenance gate and no `[cli_wrapper]` fork**, and hard-codes `HelloSpirit` —
`hello-spirit` is not one of the eight `LoadedSpiritKind` names at all.
**Consequences the dev must hold:** (a) AC1 and T2 re-point **the standalone arm only**, so the realistic
collapse is **one call site, not three** — do not price a three-copy saving; (b) `maos-bin` is at **ZERO
headroom**, so a forecast biased optimistic is the expensive direction to be wrong in; (c) treat **+420** as the
working number and **measure at T0 and again after the extraction, before asking for the raise.**

⚠ **The aggregate alarm is already FIRING** (167189 vs 158608, `kloc.toml:657`); hardfail is 170884 (`:658`),
leaving **+3695**, and this story's ×1.3 upper bound totals ≈ **+828**. State the new aggregate; do not silence
the alarm.
⚠ **CORRECTED (round 2, §17 R10/R15): the kernel row is not one line in one file.** The posture clamp lands in
**two further kernel-core files** — `capability/cap_policy/mod.rs` (the `Option<Posture>` field) and
`security/mod.rs` (two `min()`s, not one) — so the kernel change is **`+10…+16`** raw (`×1.3` upper **+21**),
THREE per-file digests move, and the re-pin is **twelve edits**. If §14 **Q4** is granted, a fourth file
(`scheduler/scheduler_loop.rs`) joins — and **§14 Q4 = Shape A was ratified, so it does join**: the kernel
change is **`+16…+26`** raw, **FOUR** per-file digests move, and the re-pin is **thirteen edits**. ⚠ And §14
**Q5 = Shape A** widens the `maos-bin` row too: the collapse is **two** call sites (standalone **and**
topology), so `+420` is a **floor**, not a ceiling. Re-measure at T0; do not price from this table.
⚠ **ROUND 3 (§18) MOVES TWO MORE ROWS.** The kernel row grows again: the `scheduler_loop.rs` edit is **three
coordinated hunks**, not one — cleanup ahead of the hook (R21 mechanism (x)), the
`hook_name == "on_unload"` guard on `check_hook_outcome`'s `handle_crash` spawn, and the inline
`task.orphaned`/`enforce_disposition` (R22) — so treat `+26` as a **floor** and measure. And **`maos-control`
and `maos-bin` each gain an unbudgeted edit** for R23's sixth key on `GET /v1/spirits/{id}`: `SpiritStatusRow`
(`maos-control/src/lib.rs:418-428`), the JSON literal (`:1024-1029`) and `spirit_status`
(`operator_door.rs:1462-1468`). `maos-control` has `+527` of headroom so it absorbs it; **`maos-bin` does not
and this is the third thing widening its raise** (topology re-point, `BinPrivateOps` widening, the sixth key).
✅ Measured and not a problem: `maos-kernel-core` headroom is **exactly ZERO**, the raise is required, and its
only tree-bound gate is `assert!(actual <= RATIFIED_AT_EASING)` (`recovery_lane_ceiling_rule.rs:275`) with a
documented same-commit escape already exercised by 16-3/16-4 and 16-5.

**Dependencies:** no new crate, no new workspace member (`check-workspace-count` PASSED 55/55 — unaffected),
no lockfile entry, **no new env var** (`check-env-contract` PASS, 94 vars — and its scope is `maos-bin/src`
only, so a `maos-cli` read would be invisible to it anyway; do not add one).
⚠ **One new file under `crates/maos-bin/src`** (`admission.rs`) ⇒ **`SCANNED_SOURCE_FILES` 23 → 24**
(`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:882`, directory-listing equality assert `:981-985`;
`ls crates/maos-bin/src/*.rs | wc -l` = 23 at HEAD). ⚠ **The epic's cite is stale for the third consecutive
story** — `epic-16-…md:53` says "22 at `:881`" with the assert at "`:967-981`".
⚠ **`verb_table_15_3.rs` is untouched** provided `load` is a **`maosctl` clap subcommand** and not a `maos`
verb and not a `MAOS_ONE_SHOT` mode: `VERBS.len() == 8` (`:137`), the two frozen per-build surface arrays
(`:161-170`), `MAOS_ONE_SHOT_MODES.len() == 32` (`:207-211`) and the frozen `--help` test (`:243`) all stay.
`maosctl --help` goes 25 → 26 surfaces; **no test pins that number.**

---

## Decisions ruled in this story (rule 7 — one decision per fork)

Rulings marked **(§14)** are recommendations pending operator ratification; every other row is ruled by
measurement recorded in §1–§6.

| # | Fork | Ruling | Rejected, and why |
|---|---|---|---|
| **D-16-6-A** | What happens to a door-loaded Spirit that is never started (§1) | **Shape B — add the `(Loaded, Unloaded)` arm to `is_transition_allowed` (`control_block.rs:448-453`). ONE kernel line, ◆ FLAG-Winston, seven artifacts. (§14 Q1 — needs the operator's grant.)** It is the only shape in which `load` is FR9's `load`: the verb that creates a state owns the state's exit. It also closes the `epic-16-retrospective` NFR-Rel-11 row **at origin** (rule 10(a)) and repairs the *existing* partial-admission leak at `main.rs:5788`, which two prior stories documented (`supervision.rs:1339-1344`, `shell_host.rs:242-247`) and neither could fix. | *Shape A — `load` also starts the Spirit* (zero kernel-Δ): collapses two of FR9's five named verbs into one, which is spec drift the founder directive forbids; leaves `maosctl start` **permanently unreachable** (§3) so FR9 is 4/5 while claiming 5/5; and `start` can itself fail, reproducing the identical orphan one transition later. *Shape C — ship `load` and document that the Spirit cannot be removed*: ships a pid/token/SCB leak and a burned name as a feature, with no operator recourse. *Shape D — a force-unload in maos-bin that removes the SCB from the map directly, at ZERO kernel-Δ*: ⚠ **this is a REAL alternative — the tree already does it twice, and `scbs()` is `pub` (`scheduler_loop.rs:148`), so maos-bin could too in three lines.** It is declined on three grounds, not on impossibility — and **R1 (§16) ruled the FIRST of them decisive; the other two support it**. (1) **Both existing bypasses are internal fault paths, not operator verbs**: `handle_crash` discards the transition `Err` (`supervision/crash_detector.rs:120-124`) and cold-swap's rollback removes outright (`lifecycle/upgrade.rs:200-203`) — a crash and an aborted upgrade are events the kernel is recovering from, whereas `unload` is a thing an operator *asks for*, and an operator-facing verb that lies to the state machine is the failure shape this epic's 16-5 spent a whole story on. (2) **They do not agree on what the Spirit gets**: `handle_crash` still revokes tokens (`:127`), writes the `terminate_spirit` receipt (`:130`) and journals (`:181-189`); cold-swap's rollback writes **nothing**. Copying either would mean choosing, in maos-bin, what NFR-Rel-11 means — a kernel invariant decided in an adapter. (3) **It makes the door's own refusal incoherent**: the door already refuses cold-swap *because* it would put an unadmitted Spirit in the map under a new pid (`operator_door.rs:632-643`), and shipping a second `transition`-bypassing mutation as the operator's `unload` would contradict that refusal in the same file. The honest version of Shape D costs **one kernel line instead of three maos-bin lines**, and buys a state machine that is true. |
| **D-16-6-B** | Where the admission logic lives | **ONE extracted function in a NEW `crates/maos-bin/src/admission.rs`, owning the whole `load → admit → start` triple plus rollback; reached from the door through a NEW `BinPrivateOps::load_spirit` method.** `BinPrivateOps` (`operator_door.rs:54`) is the established seam for bin-private composites the door does not hold — and `DoorInner` (`:177-210`) holds **neither `security` (`main.rs:2032`) nor `pid_by_spirit_id` (`:2529`) nor `shared_journal` (`:2811`)**, the three things a load needs most. Class construction reuses `successor_factory` (`main.rs:3123-3198`, match `:3130`), not a fifth copy. | *Call the kernel's `load` directly from the door* — admits at hardcoded T2 with empty capability requirements (§2), a sandbox downgrade. *Add the logic inline in `operator_door.rs`* — a second admission implementation, which AC1 forbids by name. *Widen `DoorInner` to hold `security`* — `OperatorDoor::new` already takes 19 args (`operator_door.rs:224-249`) and the `Arc::get_mut(&mut scheduler)` finalization at `main.rs:2834` constrains new holders; the seam exists precisely to avoid this. |
| **D-16-6-C** | Which run path's order the extraction adopts (§2c) | **The STANDALONE order: model-provenance → load → admit → start.** It refuses before allocating a pid, and it emits `emit_vetter_key_event` on **both** grant and rejection (`main.rs:5789`/`:5779`), where topology emits on grant only (`:4749`). The topology divergence is **recorded, not silently normalised** — §11 row 2. | *Topology's order* — runs provenance after admission, so a provenance-refused Spirit has already been admitted at a real pid. *Normalise both in this story* — re-sequencing a shipped topology path is unbudgeted churn in a story already crossing two ceilings. |
| **D-16-6-D** | Serializing a command with no Spirit id (§4) | **The handler parses the manifest, learns the id, then takes `self.spirit_lock(&spirit_id)` itself before the extracted call.** `spirit_key()` stays `None` (the id genuinely does not exist at enqueue time) but the critical section is correct. This also covers the **kernel's own non-atomic `AlreadyLoaded` guard** (`scheduler_loop.rs:268` vs `:348`) for door-initiated loads. ⚠ It does **not** cover a race against `maos run`'s own boot loads — acceptable because one daemon per `MAOS_HOME` holds the store `flock` and `maos run` loads only before serving. **Stated, not assumed.** | *Leave it unserialized like `ForgetMemory`* — reproduces the exact check-then-act shape 16-5's AC3 was written to fix, one story later. *Key `spirit_key()` off the manifest path* — two paths can name the same Spirit; the key would not serialize what it must. *Rely on the CRL `admission_guard`* — it is `None` in the default configuration (`scheduler_loop.rs:257`), so the guarantee would exist only when a CRL happens to be loaded. |
| **D-16-6-E** | Route shape (§4) | **A NEW collection arm `POST /v1/spirits` with body `{"manifest": "<canonical absolute path>"}`.** `["v1","spirits"]` currently falls to `_ => 404` (`maos-control/src/lib.rs:1211`), so there is no ordering conflict with the `{id}` / `{id}/sandbox` disambiguation the comment at `:963-966` warns about. Body shape follows `spirit upgrade`'s `target_manifest` precedent (`subcommands.rs:2313-2335`). | *`POST /v1/spirits/{id}/load`* — `validate_id` (`:1443`) forbids a path as a segment, the id is unknown before parse, and `lifecycle_command` 404s on `resolve_pid` **before** doing anything (`operator_door.rs:438-440`). *Send the manifest BYTES like `ImportRevocations`* — defensible under D-16-1-X, but the epic's AC2 explicitly requires the daemon to read the file itself (same host, same uid) so that admission sees the real file and its mode; carrying bytes would also bypass the non-regular-file refusal. |
| **D-16-6-F** | The vacuous gate (§6) | **Add exit line 5b to `epic-16-…md`'s hermetic block plus its provenance bullet, in the creation commit** (after line 5's root, BEFORE line 6's `kill` — a line after line 7's `maos purge` would have no daemon, and renumbering would invalidate every provenance bullet), so `check-exit-commands` OWES `load` to this key and rule 1 binds. Second COMMAND edit in the epic's history; `bash -n` re-run clean; any placeholder quoted. | *Leave the block alone and rely on a named CI job* — the job is necessary (AC6) but it is not the epic's stated control; rule 1 names `check-exit-commands` explicitly, and a story whose headline verb no gate can see is the shape this lane keeps re-learning. *Add the line later in dev* — the gate's owed-state is what makes the story's claim auditable from creation. |
| **D-16-6-G** | The AC3 threat statement | **Keep and extend the two statements that already exist; author no third.** `README.md:282-284` and `docs/adr/ADR-062-…md:161-165` already say a bare Worker runs as the operator uid and can read the `0600` `control.json` (`crates/maos-domain/src/operator_door.rs:37`, `spawn_and_bridge` has **no `env_clear`** at `runtime.rs:453`, and `credential_posture_2c.rs:299` forbids adding one). AC3's real obligation is that **`load` adds no new reachable surface** — no new env var, no new file — so 17-1's `MAOS_OPERATOR_` prefix probe still covers it. | *Author a new threat-model section in ADR-062* — the ADR has no such heading (`## Context :12`, `## Decision :46`, `## Consumers :82`, `## Rationale :91`, `## Consequences :105`, `## Amendment — Story 16-1 :115`); the amendment section is the right home. *Treat AC3 as a ship gate* — 16-1 already recorded that it REPLACED a gate which was satisfied when written. |

⚠ **RATIFIED AMENDMENTS (operator, 2026-09-19 — §17c).** **D-16-6-A** is **granted**, and its grant is four
kernel-core files, not one line (frontmatter `kernel_grant`; §14 Q1/Q4). Its *rejected* column also loses one
argument: §17 R8 proved `security_manager: None` (`main.rs:2793`) disables the kernel's own admission in the
daemon, so *"call the kernel's `load` directly"* fails not by admitting at **T2** but by admitting **not at
all** — a stronger refutation, and the story must state the true one. **D-16-6-C** keeps the standalone order
but its *"recorded, not silently normalised"* clause is **superseded**: Q5 = Shape A resolves the divergence in
this story (§2c, AC1, §11 row 2). **D-16-6-D** is amended by §17 R12 — the handler lock must copy
`run_command`'s bounded `timeout(lock_wait, …)` shape (`operator_door.rs:1323-1330`) and answer `spirit_busy`,
not take a bare unbounded `.lock().await` after the withdraw CAS. **D-16-6-E**'s `LONG_ROUTE_BUDGET`
justification is corrected by §17 R9/R11: admission never reaches T3 image-lock verification, `route_budget`
has a `_` arm so the arm is not compiler-enforced, and the surviving reason is the `on_load`/`on_start` hook
budgets inside the triple.

---

## Acceptance Criteria (6)

1. **AC1 (command — the operator adds a Spirit to a running daemon, through the gates `maos run` applies).**
   With a `maos run` root serving and its door bound (16-1), `maosctl load <manifest>` loads a second Spirit
   into the RUNNING root and `maosctl spirit inspect <id>` reports **`lifecycle_state: Loaded`** from
   `GET /v1/spirits/{id}` (`maos-control/src/lib.rs:1014`, five fields `:1024-1030`, state from
   `SpiritControlBlock::current_state` via `operator_door.rs:1461`).
   The path is **one extracted function** (D-16-6-B, new `crates/maos-bin/src/admission.rs`) that owns the
   whole **`load → admit → start` triple in that order** (§2 — not the epic's inverted order), called by the
   door through a new `BinPrivateOps::load_spirit` and **by `maos run`'s own standalone arm**, proving it is
   the same code. **No second admission implementation.**
   Each of these gates is proven by **one refusal vector**, each a distinct typed outcome, never a bare
   non-zero: unreadable manifest (`main.rs:4153`); unparseable TOML (`:4159`); `[cli_wrapper]` ⊕ `[class]`
   (`:4962`); missing `[class]` (`:5083`); **unknown class name** (`classify_spirit`, `:5085` — the eight-name
   bound of §5); model-provenance refusal (`:5135`); **`admit_spirit` rejection** (`:5760`); and inside it
   `EAbiTooOld`/`EAbiTooNew` (`security/mod.rs:317`/`:323`, the NFR-Maint-9 surface),
   `SandboxTierUnsupported` for T1 and T4 (`:414`/`:410`), and `T3AdmissionFailed` (`:385`).
   ⚠ **AC1 claims no trust-tier, signature, region-pin or vetting gate** — §2b proves the path has none.
   ⚠ **R3 (§16a) — ONE GATE IS ADDED, because it exists on neither path and `load` is what makes its absence
   reachable:** an **operator posture floor**, one `Option<Posture>` field beside the four floors already in
   `OperatorPolicyConfig` (`cap_policy/mod.rs:22-43`) and a `min()` at `security/mod.rs:361`, which today stores
   `allowed_max` **verbatim** (`:359-366`). Its refusal vector is §16a's falsifier, and the admission record
   carries **both** the requested and the effective ceiling.
   ⚠ **R2 (§16) — the loaded Spirit must be VISIBLE:** `GET /v1/daemon` (`maos-control/src/lib.rs:985-995`)
   returns `spirit_ids` only, so a loaded-and-forgotten Spirit is indistinguishable from a working one on the
   only surface that enumerates the daemon. It reports lifecycle state alongside the id. Nothing supervises a
   `Loaded` Spirit — the DRR picker selects only `Running` — so observability is the contract, and it is the
   operator's cue to `start` it or `unload` it.
   ⚠ **Every refusal leaves NOTHING behind**: proven by asserting the SCB map is unchanged and the pid counter's
   Spirit is absent from `GET /v1/daemon`'s `spirit_ids` (`maos-control/src/lib.rs:985-995`) after each vector.
   ⚠ **CORRECTED (round 2, §17 R9) — THREE of the vectors above CANNOT BE WRITTEN, and AC1 must not promise
   them.** `effective_sandbox_tier` (`cap_policy/mod.rs:201-224`) seeds the manifest leg from
   `manifest_scopes[pid].declared_tier`, **absent on first admission** ⇒ `SandboxTier::DEFAULT_FLOOR`, and
   `admit_spirit`'s `manifest: &SandboxConfig` parameter is read **only** for `image_pin` (`security/mod.rs:403`).
   The manifest's `[sandbox] tier` is parsed (gate #6) and **discarded** — no reader exists on the admission
   path. With `global_sandbox_floor` defaulting to `T2` and both tier-floor maps empty in production,
   `effective` is **always T2**, so **`SandboxTierUnsupported` (T4 `:410`, T1 `:414`) and `T3AdmissionFailed`
   (`:385`) are unreachable from a manifest.** AC1 therefore claims **eight** refusal vectors, not eleven, and
   **states the accept-and-ignore residual by name** (§11 row 7). Falsifier: obligation (ab).
   ⚠ **CORRECTED (round 2, §17 R10) — the R3 clamp is TWO `min()`s, not one.** `security/mod.rs:357-366` stores
   **both** `current: posture_section.default` and `allowed_max: posture_section.allowed_max` verbatim, and the
   manifest validator (`maos-manifest/src/manifest.rs:687-701`) enforces only `allowed_max >= default`. Clamping
   `allowed_max` alone stores `current > allowed_max` for orchestrator and butler — the Spirit runs **above its
   own ceiling** and the clamp clamps nothing. Both fields are `min()`-ed against the floor. `Posture`
   (`manifest.rs:643-653`) derives `Ord` least-privilege-first, so `min()` is a real clamp; the floor is
   `Option<Posture>` and defaults `None` (forced — `Posture` derives no `Default`), so **`maos run` is provably
   unchanged**. Falsifier: obligation (ac).
   ⚠ **R23 FOLDED (round 3, §18) — the clamp needs a SURFACE and a SAFE DEFAULT, and the field is misnamed.**
   (i) §16a's *"the admission record carries both the requested and the effective ceiling"* has **nowhere to
   live** at HEAD: `security/mod.rs:437-444` writes a `LifecycleEntry` (`i10.rs:121-129`) with **no posture
   field**, and `GET /v1/spirits/{id}` returns five fields (`maos-control/src/lib.rs:1024-1029`) of which
   `posture` is only `state.current` (`operator_door.rs:1452-1456`). AC1 therefore ships a **sixth key** —
   `SpiritStatusRow` (`maos-control/src/lib.rs:418-428`) + the JSON literal (`:1024-1029`) +
   `spirit_status`'s construction (`operator_door.rs:1462-1468`) — carrying the **effective** ceiling beside the
   **requested** one. Additive-safe (no key-set pin on this route); **omit it and this AC is unverifiable**
   (obligation (ao)). This adds a `maos-control` + `maos-bin` edit §10 must carry.
   (ii) **The ceiling is `None` in every fixture that reuses the 16-1 daemon root.** Set to `Cautious`, butler
   (`default = "assistive"`, `spirits/butler/manifest.toml:43`) stores `min(Cautious, Assistive)` and
   `one_daemon_one_door_16_1.rs:390-403`'s `assert_ne!(before, after)` **reds** — a shipped test, in this
   story's own harness. The clamp's vectors get their **own** root (obligation (am)).
   (iii) **Name it `operator_posture_ceiling`, not `…_floor`** — it is implemented as `min()`, i.e. a ceiling;
   a later "fix" to `max()` bricks every Spirit at `Posture::Autonomous` via `shift_posture`'s
   `NonRuntimePosture` refusal (`cap_policy/mod.rs:239-241`). Legal values are bounded to
   `{cautious, assistive, autonomous-with-halt}`.
   (iv) ⚠ **Stated, not assumed:** clamping `allowed_max` changes `posture_hash()` (`security/posture.rs:108-110`
   hashes **both** fields), so a cap token minted against a pre-clamp snapshot verifies as `PostureMismatch`.
   Benign only because admission precedes token issuance on both `maos run` arms.
   ⚠ **CORRECTED (round 2, §17 R18) — R2's field is a NEW SIBLING KEY, never a reshape of `spirit_ids`.**
   `post_surface_16_1.rs:1238` asserts `daemon["spirit_ids"][0] == "butler"`; turning the array into objects
   breaks it. `DaemonStatusRow` (`maos-control/src/lib.rs:430-439`) is **not `#[non_exhaustive]`**, so the new
   field is a compile break at three literals (`operator_door.rs:1472`, `post_surface_16_1.rs:294`,
   `submit_and_wait_16_2.rs:49`) — compiler-caught, and that is the whole migration.
   ⚠ **UNLISTED PREREQUISITE (round 2, §17 R13):** `BinPrivateOpsImpl` (`main.rs:8768-8776`) holds
   `shared_journal` but **no `security` and no `pid_by_spirit_id`**, and `trait BinPrivateOps`
   (`operator_door.rs:54-107`) declares five methods, none able to load or admit. Both must be **widened**, and
   the handler must perform the `pid_by_spirit_id` insert the standalone arm does at `main.rs:5819`.
   ⚠ **CONSTRAINT (round 2, §17 R19):** `admission.rs` is **not** whitelisted in the `manifest_scopes` negative
   at `cohort_daemon_smoke_13_5c.rs:992-995`, so the extracted function **must never touch `manifest_scopes`**.
   ⚠ **RATIFIED WIDER (operator, 2026-09-19 — §14 Q5 = Shape A; §17c).** The claim *"No second admission
   implementation"* was FALSE while only the standalone arm was re-pointed. **AC1 now re-points BOTH real
   file-manifest arms — standalone (`main.rs:5083-5828`) AND topology (`:4600-4790`) — at the one extracted
   function**, which makes three consequences binding: (i) **§2c's ordering divergence is RESOLVED, not
   recorded** — both adopt the standalone order (provenance → load → admit → start, D-16-6-C), so §11 row 2 is
   **no longer a cut line**; (ii) **topology's missing audit row is fixed** — `emit_vetter_key_event` fires on
   **rejection as well as grant**, closing the gap at the `map_err` at `:4746-4748` where today only `:4749`
   emits; (iii) re-sequencing a **shipped** path needs its own proven-red: capture topology's current
   grant-only/reject-silent behaviour RED at HEAD before the re-point. **Shell (`:3740-3856`) is DECLARED OUT OF
   SCOPE BY NAME, not counted as a copy** — it parses the compiled-in `maos_spirit_hello::MANIFEST_TOML`
   (`:3741`), hard-codes `HelloSpirit` and the id `"hello-spirit"`, never calls `classify_spirit`, runs no
   provenance gate and inserts no `pid_by_spirit_id`; `hello-spirit` is not one of the eight `LoadedSpiritKind`
   names. ⚠ **Budget consequence:** `maos-bin` is at **ZERO** headroom and this widens the change — the
   collapse is now **two** call sites, not one, so measure before and after and treat the §10 `+420` as a floor,
   not a ceiling.

2. **AC2 (the load can be undone — the kernel arm).** `is_transition_allowed`
   (`crates/maos-kernel-core/src/scheduler/control_block.rs:444-455`) gains **`(Loaded, Unloaded)`**, and:
   (a) `maosctl load <manifest>` then `maosctl unload <id>` **exits 0**, removes the SCB
   (`scheduler_loop.rs:518-521`), revokes the Spirit's capability tokens (`:514`), drains its halts (`:516`)
   and **emits the NFR-Rel-11 receipt** via `terminate_spirit` (`:505`, `halt/termination.rs:26`);
   (b) the **proven-red baseline is captured first** — the same sequence at HEAD returns
   `409 invalid_state_transition` and leaves the SCB in the map (§1's table);
   (c) loading the same id again after the unload **succeeds**, proving the name is no longer burned;
   (d) the `admit_spirit`-rejection path of AC1 now unloads its own partial load instead of leaking
   (`main.rs:5788`'s residual, routed here by `16-3-…md:4`);
   (e) an already-loaded id ⇒ typed **`409 AlreadyLoaded`** (`LifecycleError::AlreadyLoaded`,
   `scheduler_loop.rs:269-271`) — asserted under **concurrent** load of the same id, which is the D-16-6-D
   critical section, not a sequential pair;
   (f) the daemon reads the manifest itself, **canonicalised by the client** (`canonical_manifest`,
   `subcommands.rs:93-103`) and **refuses a non-regular file**.
   ⚠ This AC is the ◆ FLAG-Winston grant and the epic's `kernel-Δ 0` claim's contradiction — see §14 Q1.
   ⚠ **NEW VECTOR (g), round 2, §17 R7 — the arm makes an existing receipt gap reachable from this very verb
   pair.** `unload` flips the state at `scheduler_loop.rs:483` **before** the fallible `check_hook_outcome?` at
   `:502`, and `kernel_invocation_allowed` (`maos-spirit-abi/src/lifecycle.rs:510-515`) returns **`true` for an
   empty `enabled_hooks`**, so a default-manifest door load **does** dispatch `on_unload`. A `Panicked` or
   `BudgetExceeded` outcome (`hook_dispatch.rs:573-588`) leaves the SCB `Unloaded` **and still in the map**,
   with a `lifecycle.unload` TL row and **no `term-…` receipt** — and the operator's retry hits the
   `:479-481` early return and returns **`Ok(())`, still with no receipt**. So *"a load that succeeds can always
   be undone"* is false on the hook-failure branch, and AC2's own NFR-Rel-11 promise has a hole the arm opens.
   **RATIFIED (operator, 2026-09-19 — §14 Q4 = Shape A; §17c): AC2(g) SHIPS THE FIX, it does not merely record
   the hole.** (g) `terminate_spirit`, `revoke_all_for_pid`, `drain_for_spirit` and `spirits.remove` run **even
   when `on_unload` fails** — either by moving them ahead of the hook or by demoting the `?` at
   `scheduler_loop.rs:502` to a recorded non-fatal outcome — so **every** unload of a door-loaded Spirit emits
   its NFR-Rel-11 receipt and leaves the map clean. The falsifier (obligation (aa)) is still captured **RED
   first**: force a hook panic or budget overrun at HEAD+arm, observe the stranded `Unloaded` SCB with no
   receipt and the `Ok(())` retry, then land the repair and watch it go green. ⚠ This makes
   `scheduler/scheduler_loop.rs` the **fourth** kernel file and the fourth pinned digest, and it **closes
   `epic-16-retrospective` cluster (a) WHOLE** instead of half — §11 row 1 is no longer a cut line. ⚠ It also
   repairs the `report.failed` regression §17 R17 names: without it the sweep in `unload_all_loaded`
   (`supervision.rs:139-167`) would stop reporting the real hook failure as well as the phantom one.
   ⚠ **R21/R22 FOLDED (round 3, §18; §18a ruled (A) by the operator 2026-09-19) — the MECHANISM IS PINNED, and
   it is not either-or.** AC2(g) is **mechanism (x): `terminate_spirit` / `revoke_all_for_pid` /
   `drain_for_spirit` / `spirits.remove` move AHEAD of `fire_on_unload`**, plus three things measurement forces:
   (i) **`check_hook_outcome`'s `Panicked` arm MUST NOT spawn `handle_crash` when `hook_name == "on_unload"`**
   (`scheduler_loop.rs:573-595`). `unload` now owns revoke + `terminate_spirit` + `map.remove`, and
   `handle_crash` (`crash_detector.rs:87-200`) performs the *same* teardown — `terminate_spirit` is **not
   idempotent** (`HaltId` is `term-{spirit_id}-{pid}-{timestamp_ns}` off `SystemTime::now()`,
   `halt/termination.rs:34-45`; `boot_nonce` reaches only the body at `:65`/`:101`), so two calls mint two
   `HaltId`s and **four** `EpistemicHalt` TL frames with mismatched `kind_str`. ⚠ The weaker guard *"spawn only
   if the pid is still in the map"* **does not work** — at `:502` it still is. ⚠ `Ok(Err(join_err))` maps a
   **cancelled** hook to `Panicked` too (`hook_dispatch.rs:657-664`), so the guard is not panic-only.
   Falsifier: obligation (aj).
   (ii) **`task.orphaned` + `enforce_disposition` are performed INLINE on the `on_unload`-failure path.** Under
   (x) the spawned `handle_crash` would hit its Step-1 lookup (`crash_detector.rs:78-84`), return `NotLoaded`,
   and be swallowed by the `let _ =` at `scheduler_loop.rs:581` — so FR50's disposition and orphan frames, which
   fire today, would be **silently lost**. Falsifier: obligation (ak).
   (iii) **RECEIPT SEMANTICS, ruled (A):** the `PlannedUnload` receipt attests **the kernel's release of the
   Spirit, not the death of an OS child.** It is written before `on_unload` signals anything, and that is
   correct by construction — `TerminationKind::PlannedUnload` is already emitted for every in-process class,
   none of which has a child at all. The rejected (B) — `terminate_spirit` *after* the hook, guarded on both
   paths — reintroduces a fallible step between the state flip and the receipt, which is the exact shape §17 R7
   exists to remove. AC2 says this in one sentence rather than implying a process-death guarantee it never had.
   ⚠ Budget: (i)+(ii) make the `scheduler_loop.rs` edit larger than §14 Q4 priced — re-measure at T0.

3. **AC3 (`start` gets its first subject).** With the Spirit in `Loaded`, `maosctl start <id>` moves it to
   `Running` (`operator_door.rs:449-470` requires exactly `Loaded`) and `maosctl spirit inspect <id>` reports
   `Running`. **Proven-red at HEAD:** against a root at HEAD, `maosctl start <any running spirit>` returns
   `409` with the D-16-1-J detail string, and **no `Loaded` Spirit exists that an operator could have created**
   (§3). ⚠ The proof must be worded as *operator-creatable*, never *existent*: a failed Worker `start`
   (`supervision.rs:1334`→`:1339`) strands a durable `Loaded` SCB inside a **serving** root, so a falsifier
   asserting "no `Loaded` SCB exists at HEAD" is disprovable and must not be written. The full operator sequence
   `load → Loaded → start → Running → pause → Paused → resume → Running → unload → gone` runs end to end, which
   is **FR9's five verbs against one Spirit** for the first time.

4. **AC4 (the door surface).** A new `OperatorCommand::Load { manifest }` (`maos-control/src/lib.rs:270`) with
   arms in **all THREE exhaustive matches over the enum, plus a FOURTH arm no compiler enforces** — ⚠ the epic
   and this story's first draft said two, and round 1 said four; measured (§17 R11), the exhaustive three are:
   `spirit_key()` ⇒ `None` (`maos-control/src/lib.rs:346`, with the D-16-6-D handler-side
   lock), `DoorInner::dispatch` (`operator_door.rs:329`), and
   **`fn command_verb` (`operator_door.rs:1525-1545`) ⇒ `"load"`** — the last is not cosmetic: it is the
   machine label written into the completion TL row's intent as
   `operator.{verb}.{operation_id}:{outcome}` (`operator_door.rs:1391-1398`), so **it decides what the audit
   record says the operator did.** The fourth arm is `route_budget()` ⇒ `LONG_ROUTE_BUDGET` (`:368`, table
   `:367-381`) — **that match ends `_ => DEFAULT_ROUTE_BUDGET`, so omitting the `Load` arm compiles silently and
   the verb runs on a 10 s budget** (obligation (l) is its only control). None of the exhaustive three has a
   `_` arm, so a missing variant there fails to compile — but AC4 also asserts
   the TL row reads `operator.load.<operation_id>:completed`, which no compiler checks. Served by a new
   `POST /v1/spirits` arm (D-16-6-E)
   whose body is `{"manifest": "<canonical absolute path>"}`. Proven: bearer-before-route (401 ⇒ exit 77,
   `lib.rs:901-910`; header parse `:889-899`); `GET /v1/spirits` ⇒ 405; missing `Content-Length` ⇒ 411 (`:752`); body > 64 KiB ⇒ 413
   (`:765-770`, `MAX_BODY_BYTES` `:215`, refused from the Content-Length header alone before any read); malformed/missing `manifest` field ⇒ 400; the full **D-16-1-P exit-code matrix** via the shared
   driver `assert_typed_matrix` (`crates/maos-cli/tests/support/fixture_door.rs:471-517`) and the four
   discovery failures via `assert_discovery_failures` (`:525`). ⚠ The `maos-cli` side uses the hand-rolled
   `door_client::exchange` (`door_client.rs:259`) — **no `maos-control` dependency**
   (`crates/maos-cli/tests/dep_kernel_core_free_test.rs:8-29` is a `cargo tree` ban that counts dev-deps).
   ⚠ **CORRECTED (round 2, §17 R11) — there are THREE exhaustive matches, not four, and `route_budget` is NOT
   one of them.** `route_budget` (`maos-control/src/lib.rs:367-381`) ends `_ => DEFAULT_ROUTE_BUDGET`, so a
   missing `Load` arm **compiles silently** and the verb inherits **10 s** (`:227`) — there is no fifth match
   anywhere: the enum derives only `Debug, Clone, PartialEq, Eq` (`:270`), has no `Display`/`Serialize`/metrics
   impl, and `spirit_command` (`:1222`) matches `&str`. The exhaustive three are `spirit_key` (`:345-364`),
   `DoorInner::dispatch` (`operator_door.rs:329-420`) and `command_verb` (`:1526-1544`). **Obligation (l) is
   therefore the only control over the budget arm and is promoted to BINDING.** ⚠ And the story's stated reason
   for `LONG_ROUTE_BUDGET` is void — §17 R9 proves admission never reaches T3 image-lock verification; the
   surviving reason is the `on_load`/`on_start` hook budgets inside the triple.
   ⚠ **CORRECTED (round 2, §17 R12) — D-16-6-D's lock must copy `run_command`'s BOUNDED shape.** The port lock
   is taken at `operator_door.rs:1323-1345` with `timeout(lock_wait, lock.lock())` where
   `lock_wait = deadline − 250 ms`, **before** the `QUEUED → STARTED` CAS (`:1349-1357`). `Load`'s `spirit_key()`
   is `None`, so its handler lock runs *after* the CAS: a bare `.lock().await` would be **unbounded** on a task
   never cancelled (`:1407`), and a contended `Load` would answer **`handler_still_running`** where every other
   verb answers `spirit_busy`. AC4 asserts the loser of a contended pair gets **`spirit_busy`** within
   `lock_wait`. ✅ Separately confirmed: the lock is **port-level**, so `shell_host.rs:171`'s path is covered.

5. **AC5 (the pin moves honestly).** ⚠ **R5 — this AC is ONE VECTOR WITH ONE VERDICT, not ten assertions.**
   *"A list to copy is itself a premise"*: pull the arm ⇒ **every one of the ten must red together**; if any
   single one stays green, the list was short and you now know it. Ten named assertions that all fail for the
   same reason is a checklist wearing a lab coat (Splinter). The kernel lines are re-pinned across **all TEN edits in EIGHT artifacts** in the
   same commit (frontmatter `kernel_grant`) — including the FOUR `24628` literals in
   `kernel_pin_content_hash_16_0.rs` (`:802`, `:820`, `:829`, `:840`) and the rewritten
   `fn loaded_to_unloaded_rejected` (`control_block.rs:567-573`), **whose existence AC2 inverts**, `cargo run -p xtask -- check-kernel-baseline` PASSES at the new
   `src_lines` with **98 files** and the `scheduler/control_block.rs` digest changed **by name**, and
   `cargo test -p xtask --test kernel_pin_content_hash_16_0 --test recovery_lane_ceiling_rule
   --test d11_xtask_ceiling_ratchet` is green. ⚠ `src_lines` **stays on line 505**
   (`kernel_pin_content_hash_16_0.rs:846-851` asserts its position), and `RATIFIED_AT_EASING`
   (`recovery_lane_ceiling_rule.rs:268`) moves in the same commit or
   `kernel_core_ceiling_has_not_moved_under_the_easing` reds. The `maos-bin` ceiling raise is **measured after
   `cargo fmt --all`**, cites the bare token `16-6`, and lands with its driver.
   ⚠ **CORRECTED (round 2, §17 R15) — AC5 is UNDERSTATED as written and must name THREE digests.** With R3 in
   scope the digests that change by name are `capability/cap_policy/mod.rs` (`kernel-core-baseline.toml:548`),
   `scheduler/control_block.rs` (`:601`) and `security/mod.rs` (`:614`) — a dev satisfying the old wording
   passes while two kernel files silently change, which is the D-16-0-F failure mode the per-file map was built
   to catch. The edit count is **TWELVE**, `PINNED_ENTRIES` stays **98**, and the HISTORY row **MUST be
   line-count-neutral** or `kernel_pin_content_hash_16_0.rs:842-851` reds on the line position even with a
   correct `src_lines` value (every prior re-pin prepended a multi-line block; 16-5's is `:496-504`).
   Falsifier: obligation (ae). ⚠ `src_lines` is `24628` at HEAD and the four literals are at `:802`, `:820`,
   `:829`, `:840`; `RATIFIED_AT_EASING` is `19_053`; `kloc.toml` carries **ceilings only** — there is no
   `current` key, so *headroom* is always a `kloc-check --json` measurement, never a file read.

6. **AC6 (the gate binds).** ✅ **The epic half is DONE in the creation commit** — line **5b** (between lines 5
   and 6, before the `kill`) runs the SIX-verb sequence `load` → `spirit inspect` → **`unload` from `Loaded`**
   → re-`load` → `start` → `unload`, with its provenance bullet naming all four HEAD reds (D-16-6-F); `bash -n` is clean over all 8
   lines and `check-exit-commands` PASSES with **31 resolved** and `load` **owed to `16-6` [ready-for-dev]**.
   **The dev half:** that same gate must report `load` **resolved** (not owed) once the verb ships, and the
   whole line must execute green. A **named CI job** `maosctl-load-over-the-door` runs
   `cargo test --locked -p maos-bin --test maosctl_load_16_6 -- --test-threads=1`, copying the
   `one-daemon-one-door` shape verbatim (`.github/workflows/discipline.yml:3640-3655`) and enrolled in
   `v1-0-ship-gate.needs` **but NOT `aggregate.needs` or `EXPECTED_GATES`** (16-1's precedent, recorded at
   `:3657-3661`). ⚠ **Not 16-4's precedent** — that test has no job at all (§6).
   `SCANNED_SOURCE_FILES` moves 23 → 24 (`cohort_daemon_smoke_13_5c.rs:882`) in the same commit as
   `admission.rs`.
   ⚠ **R24 FOLDED (round 3, §18) — the template does NOT build `maosctl`, so copy it with ONE STEP ADDED.**
   `one-daemon-one-door`'s only build step is `cargo build --locked -p worker --bin worker-cli-fixture`
   (`discipline.yml:3643-3655`), but `one_daemon_one_door_16_1.rs:54-68` resolves `maosctl` beside
   `CARGO_BIN_EXE_maos` and **hard-asserts `is_file()`**. `maos-cli` is a plain `[dependencies]` entry of
   maos-bin (`crates/maos-bin/Cargo.toml:44`, section header `:39`), so cargo builds its **lib**, never its
   `[[bin]] maosctl` (`crates/maos-cli/Cargo.toml:10-12`) — the existing job is green only because
   `Swatinem/rust-cache@v2` restores a `target/` some other job populated. **The 16-6 job adds
   `cargo build --locked -p maos-cli --bin maosctl`.** Falsifier: obligation (an). ⚠ The 16-1 job carries the
   same latent defect — **report it, do not inherit it** (§11 row 8).
   ✅ **And the enrollment does block per-commit** — measured, not assumed: `discipline.yml:3-7` is
   `push`/`pull_request` on `main` with no ref guard, `v1-0-ship-gate` carries only `if: always()` (`:3729`) and
   exits 1 on any failed/skipped/cancelled need (`:3780-3782`), and **`aggregate.needs` contains
   `v1-0-ship-gate`** (`:3928`) with its own `exit 1` (`:4155-4158`). ⚠ RISK recorded: omission from
   `EXPECTED_GATES` is **silently allowed** — `check_ship_gate_completeness` iterates `EXPECTED_GATES`, never
   `needs` (`xtask/src/check_ship_gate_completeness.rs:212-230`), and nothing asserts every workflow job is
   enrolled somewhere.

---

## Review obligations (§A6)

**(a)** Read `is_transition_allowed` (`control_block.rs:444-455`) and confirm independently that
`(Loaded, Unloaded)` is absent at HEAD and that `(Loaded, Running)` is present — the asymmetry is the whole
story. **(b)** Run AC2(b)'s falsifier **at HEAD first** ⇒ it must red with `409 invalid_state_transition`; a
transition test never seen red is not a control. **(c)** After the arm lands, delete it again ⇒ AC2(a) reds and
AC1 still passes — proving the two ACs are separable and the grant buys exactly one thing. **(d)** Confirm the
SCB is gone from `scheduler.scbs()` **and** from `GET /v1/daemon`'s `spirit_ids` after unload, not merely that
the verb exited 0. **(e)** Confirm the NFR-Rel-11 receipt exists by reading the TL for the
`term-{spirit_id}-{pid}-…` marker (`halt/termination.rs:42-82`), not by asserting `terminate_spirit` was
called. **(f)** Re-derive the admission order yourself on all three copies (`main.rs:3800→3813`,
`:4610→4729`, `:5181→5761`) and confirm the epic's AC1 order is inverted. **(g)** Point the extracted function
at the kernel's `load` directly instead ⇒ confirm a T3-requiring manifest is admitted at **T2**
(`scheduler_loop.rs:277-323` hardcodes `SandboxTier::T2` and empty `CapabilitiesRequired`); recorded as why
D-16-6-B exists. **(h)** Grep `main.rs:4094-6047` case-insensitively for
`trust_tier|signature|verify|region|vetted|attestation|keyring` ⇒ **zero** hits; confirm AC1 claims no gate the
path lacks. **(i)** Run `maosctl start <a Running spirit>` against a HEAD root ⇒ 409 with the D-16-1-J string; then
**force a failed Worker `start`** and confirm a durable `Loaded` SCB DOES appear in a serving root
(`supervision.rs:1334`→`:1339`, door bound at `main.rs:3362` before the run block at `:4094`). The AC3 claim
that survives is *operator-creatable*, not *existent* — a reviewer who accepts "no `Loaded` SCB exists at
HEAD" has accepted a disproved claim. **(j)** Drive AC2(e)
with two **concurrent** loads of the same id (`std::sync::Barrier::new(2)`, the shape at
`crates/maos-bin/tests/shell_host_16_2.rs:1044`) ⇒ exactly one 200 and one `409 AlreadyLoaded`; then remove the
D-16-6-D handler lock ⇒ confirm two SCBs under different pids appear, proving the kernel guard
(`scheduler_loop.rs:268` vs `:348`) is not atomic on its own. **(k)** Confirm `revocation_rules` is `None` in
the test configuration, so the `admission_guard` `tokio::sync::Mutex` (`revocation/rules.rs:33-35`) is **not**
what makes (j) pass. **(l)** Delete the `route_budget()` `Load` arm ⇒ confirm a T3 admission exceeds the 10 s
default and the route answers `503 handler_still_running`; recorded. **(m)** Enumerate every exhaustive
`OperatorCommand` match in the tree after adding `Load` ⇒ confirm there are **FOUR** (`spirit_key`
`maos-control/src/lib.rs:346`, `route_budget` `:368`, `dispatch` `operator_door.rs:329`, `command_verb`
`:1526`), that none has a `_` arm, and that **`shell_host.rs:171`'s path** was considered. Then read the
completion TL row and confirm its intent is literally `operator.load.<operation_id>:completed` — the compiler
proves the arm exists, nothing proves the **label** is right, and that label is what the audit record says the
operator did. **(m2)** Search for any OTHER removal of an SCB from the `spirits` map ⇒ there are exactly three
writes tree-wide (`scheduler_loop.rs:347` insert, `:520` remove, `lifecycle/upgrade.rs:197`/`:203`
insert/remove); confirm cold-swap's `:203` bypasses `transition` and therefore emits no receipt — the D-16-6-A
Shape D precedent. **(n)** Send a relative
manifest path straight to the door ⇒ confirm it resolves in the **daemon's** CWD (D-16-1-X) and that the client
canonicalises first (`subcommands.rs:93-103`); also send a directory and a FIFO ⇒ non-regular-file refusal.
**(o)** Confirm `maos run` itself does **not** canonicalise (`main.rs:4152`) and that the extracted function
accepts an already-canonical path from both callers without a second `canonicalize`. **(p)** Run each AC1
refusal vector and confirm the SCB map is byte-identical before and after — a gate that refuses but leaks is
the §1 defect wearing a different hat. **(q)** Re-derive `SCANNED_SOURCE_FILES` from
`ls crates/maos-bin/src/*.rs | wc -l` ⇒ 24 after `admission.rs`, and confirm the equality assert at
`cohort_daemon_smoke_13_5c.rs:981-985` moved. **(r)** Run `check-exit-commands --json` **before** the verb
ships ⇒ `load` appears in `owed` against `16-6`; **after** ⇒ it appears in `tokens_resolved` and `owed` is back
to the three 18-2/20-1 entries. **(s)** Re-run `bash -n` on the whole edited exit block and confirm no
unquoted `<placeholder>` was introduced (`check_exit_commands.rs:849-877`). **(t)** Raise `kloc.toml:247`
without moving `RATIFIED_AT_EASING` ⇒ confirm `kernel_core_ceiling_has_not_moved_under_the_easing`
(`recovery_lane_ceiling_rule.rs:266-281`) reds; recorded as the seventh artifact nobody had counted. **(u)**
Insert a HISTORY block **above** `src_lines` in `kernel-core-baseline.toml` ⇒ confirm
`kernel_pin_content_hash_16_0.rs:846-851` reds on the line position even with a correct value. **(v)** Confirm
`check-workspace-count` (55/55) and `check-env-contract` (94 vars) are untouched, and that no typed error
leaked into `maos-domain` (ZERO headroom) — if one did, it is an uncosted raise and must be named.

⚠ **(m2) IS CORRECTED BY §17 R16.** The `spirits`-map writes are **SIX, not three**: `scheduler_loop.rs:348`
insert, `:520` remove; `upgrade.rs:198` insert, `:203` remove, **`:208` re-insert**; and
**`crash_detector.rs:193` `map.remove`**. `scbs()` is `pub` (`scheduler_loop.rs:148`) with ~30 external call
sites. The obligation stands but its expected count is six, and it must also confirm that **`upgrade.rs:164`**
— cold-swap unloading the *predecessor*, a caller the story never listed — changes behaviour when that
predecessor was loaded-but-never-started. ⚠ **(l) is promoted to BINDING** (§17 R11: `route_budget` has a `_`
arm, so nothing else can catch a missing `Load` budget). ⚠ **(g) is RE-AIMED** (§17 R8:
`security_manager: None` at `main.rs:2793` means the kernel's internal `admit_spirit` never runs in the daemon,
so pointing the extracted function at the kernel's `load` admits **not at T2 but not at all** — confirm *that*,
not a tier downgrade). **Obligations (aa)–(ai) are in §17d and are part of this net.**

---

## 11. Declared cut lines (rule 10 — each names an owner, never a bucket)

| # | Item | Measured evidence | Destination |
|---|---|---|---|
| 1 | **The `on_unload`-hook-failure receipt gap, and its FALSE SUCCESS on retry** | `scheduler_loop.rs:501-512`: `check_hook_outcome?` (`:502`) runs **before** `terminate_spirit` (`:505`), so a failing hook aborts all five cleanup steps — but the state already flipped to `Unloaded` at `:483`, so a **retried** unload hits `:478-481` and returns **`Ok(())`** with no receipt, the SCB still in the map and tokens still live | **`epic-16-retrospective`** — it is cluster (a)'s *other* half (`sprint-status.yaml:253`) and D-16-3-P already records the `BudgetExceeded` shape. This story closes the `Loaded` half at origin; the hook half is a different defect in the same function and has no `load` trigger |
| 2 | **Topology and standalone disagree on admission ordering** | §2c: provenance before `load` at `:5135` vs after `admit_spirit` at `:4757`; vetter event on grant-and-rejection at `:5789`/`:5779` vs grant-only at `:4749` | **`epic-16-retrospective`** — D-16-6-C adopts the standalone order for the extracted function and records the divergence; re-sequencing the shipped topology path is unbudgeted churn in a story crossing two ceilings |
| 3 | **`ColdSwap` starts an unadmitted successor, and its rollback removes a `Loaded` SCB by raw map mutation** | `crates/maos-kernel-core/src/lifecycle/upgrade.rs:162-205` allocates a pid and inserts an SCB **bypassing `admit_spirit` entirely** (`:194-198`); on `start` failure `:203` does `map.remove(&new_pid)` **bypassing `transition`**, so no `lifecycle.unload` row, no `on_unload`, **no NFR-Rel-11 receipt**, no token revocation, no halt drain. `operator_door.rs:632-643` refuses the whole verb over the door as *"a kernel byte routed to the Epic-16 retrospective"* | **`epic-16-retrospective`** — already routed there by 16-1. ⚠ Noted twice over: AC1's extracted function is **the fix shape** if that row is taken up (cold-swap's successor could go through it), and `:203` is the **shipped precedent for D-16-6-A's rejected Shape D** — the reason this story will not add a second `transition`-bypassing removal. Not pulled in: rule 10 routes by charter, and this story's charter is `load`, not `upgrade` |
| 4 | **`requirements-inventory.md`'s FR9 homing is stale twice** | `:437` homes FR9 to E1a+E5; `:547` homes `FR9/FR13/FR16/FR24/FR51` to `16-1`. Neither reflects the 2026-09-13 split | **NOT CUT — repaired here** (§12), because this story is FR9's closing vehicle and a reader following the inventory is sent to a story that does not own `load` |
| 5 | **The epic's FR9 cite is off by one** (`prd/functional-requirements.md:37` is **FR8**) | `:38` is FR9; `:37` is FR8's `min_substrate_version` clause | **NOT CUT — repaired here** (§12) |
| 6 | **`maos_uninstall_16_4` has no CI job** | `grep -n "maos_uninstall_16_4\|purge" .github/workflows/discipline.yml` ⇒ nothing; it runs only inside `workspace-test-suite` (`:3559`), contradicting `epic-16-…md:25` | **`epic-16-retrospective`** — a gate-binding defect in a `done` story, not this story's charter. AC6 declines to copy the precedent and says so |
| 7 | **The manifest's `[sandbox] tier` is parsed and discarded — accept-and-ignore** | §17 R9: `effective_sandbox_tier` (`cap_policy/mod.rs:201-224`) seeds the manifest leg from `manifest_scopes[pid].declared_tier` (absent on first admit ⇒ `DEFAULT_FLOOR`); `admit_spirit`'s `manifest: &SandboxConfig` is read only for `image_pin` (`security/mod.rs:403`); no reader of `SandboxConfig.tier` exists on the admission path. A manifest declaring `tier = "T3"` or `"T4"` is admitted at **T2** | **`epic-16-retrospective`** — it is a `maos run` defect 16-6 merely makes visible, and repairing it would change the effective tier of every shipped Spirit (a privilege *and* a sandbox change) in a story already crossing two ceilings. ⚠ **AC1 states the bound instead of claiming the vectors** (obligation (ab)); this row is the successor, named, not a bucket |
| 8 | **`one-daemon-one-door` (16-1) passes for a reason unrelated to what it asserts** | §18 R24: the job builds only `worker-cli-fixture` (`discipline.yml:3643-3655`) while `one_daemon_one_door_16_1.rs:54-68` hard-asserts `maosctl` exists; `maos-cli` is a plain `[dependencies]` entry (`crates/maos-bin/Cargo.toml:44`) so cargo builds its lib only. Green today via `Swatinem/rust-cache@v2` restoring another job's `target/` | **`epic-16-retrospective`** — a CI defect in a `done` story. 16-6 adds the missing build step to **its own** job (AC6) and declines to inherit the template's bug |
| 9 | **`terminate_spirit` is not idempotent** | §18 R22: `HaltId::new(format!("term-{spirit_id}-{spirit_pid}-{timestamp_ns}"))` (`halt/termination.rs:44-45`) off `SystemTime::now()` (`:34-37`); `boot_nonce` reaches only the receipt body (`:65`, `:101`). Two calls ⇒ two `HaltId`s, four `EpistemicHalt` frames, mismatched `kind_str`, no dedup | **`epic-16-retrospective`** — 16-6 makes the double call *unreachable* via AC2(g)(i)'s guard rather than making the function idempotent, because a `(pid, boot_nonce)`-keyed id is a receipt-grammar change with its own consumers (`halt/termination.rs:42-82`, SIEM readers). Named, with its falsifier (obligation (al)) |

**Not cut (EFFORT, not SCOPE):** the kernel arm, the whole load→admit→start extraction with rollback, every AC1
refusal vector, the concurrent-load critical section, the collection route, all seven re-pin artifacts, the exit
line and the named CI job. Each is required for one of this story's own ACs to be true.

✅ **ROUND 2 CLOSED TWO ROWS AT ORIGIN (operator rulings, 2026-09-19 — §17c).**
**Row 1 — the `on_unload`-hook-failure receipt gap — is NO LONGER CUT.** It claimed *"a different defect in the
same function and has no `load` trigger"*; §17 R7 disproved the second half (the arm this story adds is exactly
what routes a door-loaded, never-started Spirit into `fire_on_unload` `scheduler_loop.rs:501` and
`check_hook_outcome?` `:502`). **§14 Q4 = Shape A**, so AC2(g) ships the cleanup-ordering repair and
`epic-16-retrospective` cluster (a) closes **WHOLE**, not half. A cut line whose trigger this story ships was
never a cut line.
**Row 2 — topology vs standalone admission ordering — is NO LONGER CUT.** **§14 Q5 = Shape A**, so AC1
re-points both real file-manifest arms at the one extracted function: the §2c divergence is resolved and
topology's grant-only vetter event gains its rejection row. Rows 3–7 stand as written, each with a named owner.

---

## Dev notes

**Reuse, do not reinvent.**
- **The class construction switch already exists four times over.** `successor_factory`
  (`crates/maos-bin/src/main.rs:3125-3199`) is a `class.name.as_str()` match producing `Arc<dyn AnySpiritObj>`
  for all 8 classes, already `'static` and Arc-captured (`:3110-3122`). Build the extraction on it.
- **The seam:** `BinPrivateOps` (`crates/maos-bin/src/operator_door.rs:54-107`), impl `BinPrivateOpsImpl`
  (`main.rs:8768` struct, `:8780` impl), constructed `:3330-3338`, handed to `OperatorDoor::new` as **arg 18 of 19** (`:3339-3359`). Add one method; do not widen `DoorInner`.
- **The door test fixture:** `struct Root` (`crates/maos-bin/tests/one_daemon_one_door_16_1.rs:71`),
  `Root::spawn` `:88` (isolated `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, runs `maos init`, spawns `maos run` in
  replay, then `await_running` `:147` — readiness is a `lifecycle_state` read, **never** a successful connect),
  `ctl` `:163`, `inspect` `:196`, `lifecycle_state` `:211`, `tl_rows` `:243`, `terminate` `:275`. This is AC1's
  and AC3's harness.
- **The CLI contract fixture:** `crates/maos-cli/tests/support/fixture_door.rs` — `FixtureDoor::spawn` `:100`,
  `spawn_scripted` `:111`, `spawn_dead` `:117`, `requests()` `:211`, and the shared drivers
  **`assert_typed_matrix` `:471-517`** and **`assert_discovery_failures` `:525`**. AC4 re-runs both; do not
  hand-roll the matrix.
- **The body idiom:** `serde_json::to_vec(&serde_json::json!({ … })).expect("serializing a hand-built Value
  cannot fail")` — nine existing sites; the closest template is `spirit upgrade`
  (`subcommands.rs:2313-2335`, canonical path in the body + `LONG_ROUTE_BUDGET`).
- **Path handling:** `fn canonical_manifest` (`subcommands.rs:93-103`) — exists ⇒ else exit 1; canonicalize ⇒
  else exit 1. Never re-derive.
- **The race-test shape:** `std::sync::Barrier::new(2)` — `crates/maos-bin/tests/shell_host_16_2.rs:1044`.
- **The re-pin:** `cargo run -p xtask -- check-kernel-baseline --emit-pin`; HISTORY shape is `471608ea`'s.
- **The CI job shape:** copy `one-daemon-one-door` verbatim (`.github/workflows/discipline.yml:3640-3655`),
  including the `worker-cli-fixture` build step and `--test-threads=1`.

**Do not.**
- Do not write a second admission implementation, and do not call `SpiritSchedulerAdapter::load` from the door
  without the maos-bin gates — the kernel admits at hardcoded **T2** with empty capability requirements
  (`scheduler_loop.rs:277-323`).
- Do not auto-start on load (D-16-6-A Shape A) — it collapses two FR9 verbs and leaves `maosctl start`
  unreachable forever.
- Do not route `load` through `POST /v1/spirits/{id}/{verb}` — `validate_id` forbids a path segment and
  `lifecycle_command` 404s before doing anything (`operator_door.rs:438-440`).
- Do not rely on `spirit_key()` to serialize the load — it is `None` by construction; take the lock in the
  handler (D-16-6-D).
- Do not rely on the CRL `admission_guard` for uniqueness — it is `None` by default (`scheduler_loop.rs:257`).
- Do not add a `maos` verb or a `MAOS_ONE_SHOT` mode — `verb_table_15_3.rs` freezes `VERBS.len() == 8` (`:137`),
  both per-build surface arrays (`:161-170`), `MAOS_ONE_SHOT_MODES.len() == 32` (`:207-211`) and the `--help`
  layout (`:243`), in **both** feature builds.
- Do not add a new crate (`check-workspace-count` 55/55) or a new env var (`check-env-contract`, 94 vars).
- Do not let a typed error leak into `maos-domain` — **ZERO headroom** (`kloc.toml:395`); reuse
  `OperatorOutcome` (`maos-control/src/lib.rs:384-399`).
- Do not move `src_lines` off line 505 in `kernel-core-baseline.toml`
  (`kernel_pin_content_hash_16_0.rs:846-851`).
- Do not silence the aggregate alarm.

### Project Structure Notes

**One new file under `crates/maos-bin/src`** — `admission.rs` — so **`SCANNED_SOURCE_FILES` moves 23 → 24**
(`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:882`, entry + array length + doorbell comment + the
directory-listing equality assert at `:981-985`, all in the same commit). Declare it in
`crates/maos-bin/src/lib.rs` beside `pub mod operator_door;` (`:41`). **No new crate, no new workspace member,
no lockfile entry, no new env var.** New test files belong under each crate's `tests/` (**not** charged by
`kloc-check`); inline `#[cfg(test)]` **is** charged. The kernel edit is a single arm inside an existing
`matches!` in a file already in the pinned 98-entry set, so it reds `check-kernel-baseline` **by name** —
which is the point of AC5. ⚠ **Test isolation:** every test that spawns a door root must isolate `HOME`
(D-16-1-Q; CI plants a decoy `control.json` on `HOME` at `discipline.yml:3609-3636`), and `Root::spawn`
already does.

### References

⚠ **Every cite below is pinned at `471608ea`.** This story's rule-9 amendment to `epic-16-…md` is APPLIED in
the creation commit, so line numbers in that file shift in the working tree. Re-derive against the commit, not
the tree.

- [Source: `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md`] — §16-6 `:188-194`,
  stories-table row `:72`, *Closes* `:21`, Kernel-Δ `:53`, Kloc asks `:55`, confidence rules `:57`,
  Dependencies `:198`, exit block `:27-35`, provenance list `:38-44`, R7's cut line to 16-6 `:157`.
- [Source: `_bmad-output/implementation-artifacts/16-1-daemon-post-surface-and-verb-retarget.md`] — §15 F-B
  `:917` (which created this key), F-D `:919`, D-16-1-T `:353`, D-16-1-S `:352`, §11 rows 6 `:603` and 8
  `:605`, §12 rows 19-20 `:871-872`, §14 `:1264-1265`, §16 `:958`.
- [Source: `_bmad-output/implementation-artifacts/16-3-subprocess-crash-to-handle-crash.md`] — `blocks:` front
  matter `:4` (the pre-`start`/partial-load residual routed here by name).
- [Source: `_bmad-output/implementation-artifacts/sprint-status.yaml`] — this key `:252`,
  `epic-16-retrospective` `:253` (cluster (a) is AC2's origin), epic-16 `:240`, 17-1 `:255`.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md`] — **FR9 `:38`** (not `:37`, which
  is FR8), FR11 `:40`, FR38 `:88`, FR49 `:43`, FR51 `:55`, FR55 `:69`.
- [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md`] — NFR-Maint-9 `:140`.
- [Source: `docs/adr/ADR-062-mutating-operator-surface.md`] — `## Amendment — Story 16-1` `:115`, residual
  threat `:161-165`, gate `:3`. [Source: `docs/adr/ADR-060-spirit-forms-by-trust-tier.md`] — the
  `rust-inproc` = compiled-in first-party bound behind §5.
- [Source: `README.md`] — operator surface `:239-284`, residual-Worker threat `:282-284`.
- [Source: `xtask/kloc.toml`] — RECOVERY-LANE CEILING RULE `:49-95`, `maos-kernel-core` `:247`, `maos-domain`
  `:395`, `maos-control` `:454`, `maos-cli` `:459`, `maos-bin` `:477`, alarm `:657`, hardfail `:658`.
- [Source: `xtask/kernel-core-baseline.toml`] — `src_lines` `:505`, `[kernel_src]` `:538`, `set_hash` `:540`,
  `[kernel_src.files]` `:542-640`.

---

## Tasks / Subtasks

- [x] **T0 — re-measure; trust nothing in this file** (all ACs)
  - [x] `git status --short` clean; record HEAD. ⚠ **FIRST: `cargo build -p maos-bin -p maos-cli --bins`.**
        `Cargo.toml:60` is `default-members = []`, so a bare `cargo build` builds NOTHING, and
        `check-exit-commands` resolves verbs from **live `--help`** — on an unbuilt tree it reports
        `passed:false` with ~18 `unresolved-verb` findings and `surfaces {maos:0, maosctl:0}`. That is a tooling
        artifact, not a red, and it will waste an hour if you meet it cold.
        Then run `kloc-check --json`, `check-kernel-baseline`,
        `check-exit-commands --json`, `check-env-contract`, `check-workspace-count`, and
        `cargo test -p xtask --test recovery_lane_ceiling_rule --test kernel_pin_content_hash_16_0
        --test d11_xtask_ceiling_ratchet`. **All are green at HEAD with the binaries built** — any red after
        your first commit is yours.
  - [x] Re-measure this counts box; every number is load-bearing: run block **`main.rs:4094-6047`** ·
        `scheduler.load` sites **20** (15 in the three real copies — 1 shell + 6 topology + 8 standalone — plus 5 smoke arms) · `classify_spirit` **`:456`**, classes
        **8** · `?`/`return` in the run block **127** (103 + 24), of which **≈89** exit `main` without teardown ·
        `SCANNED_SOURCE_FILES` **23** · pin `src_lines` **24628** at `:505`, **98** files ·
        `RATIFIED_AT_EASING` **19_053** · aggregate **167189**, hardfail **170884** · `maos-bin` headroom
        **0** · `maosctl` surfaces **25**.
  - [x] **Capture AC2(b)'s proven-red FIRST**: build a HEAD root, force a `Loaded`-but-unstarted SCB (the
        `admit_spirit`-rejection path at `main.rs:5788` reaches it), call `unload` ⇒ record the
        `409 invalid_state_transition` and that the SCB survives in `scheduler.scbs()`.
  - [x] **Capture AC3's proven-red FIRST**: `maosctl start <a Running spirit>` ⇒ 409 with the D-16-1-J string;
        record that no `Loaded` Spirit exists at HEAD.
  - [x] Confirm `check-kernel-baseline --emit-pin` round-trips byte-identical **before** writing any kernel
        line — AC5 depends on it entirely.
- [x] **T1 — the epic amendment, as its own commit, BEFORE any code** (§12, D-16-6-F) — the Round-10 paragraph,
      the stories-table row, the §16-6 section, the Kernel-Δ and Kloc-asks paragraphs, the Dependencies line,
      **exit line 5b + its provenance bullet**, and the `requirements-inventory.md` FR9 repairs. Re-run
      `bash -n` on the block and `check-exit-commands --json` ⇒ `load` **owed to `16-6`**.
- [x] **T2 — extract the admission function** (AC1, D-16-6-B/C) — ⚠ **scope RATIFIED WIDER: §14 Q5 = Shape A**
  - [x] New `crates/maos-bin/src/admission.rs`; declare in `lib.rs:41`; bump `SCANNED_SOURCE_FILES` 23 → 24
        with its doorbell comment in the same commit.
  - [x] Own the whole **load → admit → start** triple in the standalone order (provenance first), built on
        `successor_factory` (`main.rs:3123-3198`, match `:3130`), with the rollback `main.rs:5788` lacks.
  - [x] Re-point **BOTH** `maos run`'s **standalone** arm **and its topology** arm (`main.rs:4600-4790`) at it,
        so the door and both roots share one implementation (§14 Q5 = A). Capture topology's current
        grant-only/reject-silent vetter behaviour **RED at HEAD first** — it is a shipped path — then make
        `emit_vetter_key_event` fire on **rejection as well as grant** (the `map_err` at `:4746-4748`).
        **Shell (`:3740-3856`) is out of scope, declared by name.** Measure `maos-bin` before and after; the
        collapse is two call sites, so `+420` is a floor.
  - [x] One refusal vector per gate in §2b, each a distinct typed outcome; assert the SCB map is unchanged
        after each.
- [x] **T3 — the kernel edits and the THIRTEEN edits across EIGHT artifacts** (AC2, AC5, D-16-6-A) — ✅ **§14 Q1
      GRANTED 2026-09-19; FOUR kernel files (Q4 = Shape A).** Add `(Loaded, Unloaded)`; add the
      `Option<Posture>` floor to `OperatorPolicyConfig` and `min()` **both** `current` and `allowed_max` at
      `security/mod.rs:357-366`; make `unload`'s cleanup run even when `on_unload` fails
      (`scheduler_loop.rs:471-523`); **rewrite `fn loaded_to_unloaded_rejected`
      (`control_block.rs:567-573`) — it asserts the behaviour you just inverted and WILL red**; `cargo fmt
      --all`; then `kloc.toml:247`, `RATIFIED_AT_EASING` (`recovery_lane_ceiling_rule.rs:268`), `src_lines`
      (staying on line **505**), **FOUR** per-file digests (`control_block.rs` `:601`, `cap_policy/mod.rs`
      `:548`, `security/mod.rs` `:614`, `scheduler_loop.rs`), the aggregate `set_hash` (`:540`),
      **all FOUR `24628` literals** in `kernel_pin_content_hash_16_0.rs` (`:802`, `:820`, `:829`, `:840`), and a
      **line-count-neutral** HISTORY row — one commit. Re-count from the tree first; do not trust this list.
- [x] **T4 — the door surface** (AC4, D-16-6-D/E) — `OperatorCommand::Load`, the **three** exhaustive arms
      (`spirit_key`, `dispatch`, `command_verb`) **and** the compiler-unenforced `route_budget` arm (§17 R11), the
      `["v1","spirits"]` POST arm, `BinPrivateOps::load_spirit` + its `main.rs:8780` impl, and the handler's
      own `spirit_lock` after the parse.
- [x] **T5 — the CLI verb** (AC4) — `Load(LoadArgs)` in `cli.rs`, dispatch, `canonical_manifest`, D-16-1-P
      mapping; re-run `assert_typed_matrix` and `assert_discovery_failures`.
- [x] **T6 — the sequence and the gate** (AC3, AC6) — `crates/maos-bin/tests/maosctl_load_16_6.rs` driving
      `load → start → pause → resume → unload` on a live root via `Root`; the named CI job; enrol in
      `v1-0-ship-gate.needs` only.
- [x] **T7 — close out** — `cargo fmt --all`, `kloc-check --json` (state the new aggregate; do not silence the
      alarm), the measured `maos-bin` raise citing `16-6`, full workspace regression, and the §A6 review net.
- [x] **T8 — round-2 corrections that are NOT optional** (§17) — the clamp is **two** `min()`s
      (`security/mod.rs:357-366`, R10); the re-pin is **thirteen** edits and **four** digests with a
      **line-neutral** HISTORY row (R15); `BinPrivateOps` + `BinPrivateOpsImpl` gain `security` and
      `pid_by_spirit_id` (R13); the `Load` lock copies `run_command`'s bounded `timeout(lock_wait, …)` shape and
      answers `spirit_busy` (R12); the `route_budget` arm is uncompiled-checked so obligation (l) runs (R11);
      AC1 drops the T1/T4/T3 vectors and states the accept-and-ignore bound (R9); `admission.rs` never touches
      `manifest_scopes` (R19); `successor_factory` gains the `researcher` arm **plus a parity test against
      `classify_spirit`** (R20); and the three production/test comments that the arm turns false
      (`supervision.rs:1339-1342`, `shell_host.rs:241-247`, `shell_host_16_2.rs:545-547`) are rewritten in the
      same commit (R17).
- [x] **T9 — round-3 blockers; NONE is optional** (§18) — mechanism **(x)** only, plus the
      `hook_name == "on_unload"` guard on `check_hook_outcome`'s `handle_crash` spawn (R21) and the inline
      `task.orphaned`/`enforce_disposition` (R22); the posture field is named **`operator_posture_ceiling`**
      with bounded values, the 16-1 fixture keeps it `None`, and the effective ceiling gets its **sixth key** on
      `GET /v1/spirits/{id}` (R23); the new CI job adds `cargo build --locked -p maos-cli --bin maosctl` (R24).
      Obligations (aj)–(ao) all run.

---

## 12. Epic OLD→NEW edits (rule 9 — APPLIED at story creation, 2026-09-18)

Every OLD anchor was verified to exist exactly once in the file named before replacement; no row replaces a pin
value or a line number that is data.

| # | File / location | OLD (anchor) | NEW (summary) |
|---|---|---|---|
| 1 | epic-16 header, before `**Wave / duration:**` | — | NEW **Round 10** paragraph: 16-6 re-derived at `471608ea`; premises in all three ACs disproved; the `(Loaded, Unloaded)` gap makes `kernel-Δ 0` false; pointer to this file |
| 2 | epic-16 stories table 16-6 row | `` FR9 (`load`) · kernel-Δ 0 · maos-bin +250–450 (admission extraction), maos-cli +40–80, maos-control +20–40 `` | ⚠ R10 measured re-book: **kernel-Δ +1 ◆ FLAG-Winston** (the `(Loaded, Unloaded)` arm), maos-bin ≤ **+546** (**ZERO headroom** ⇒ measured raise), maos-control ≤ +117, maos-cli ≤ +111, maos-journey-test ≤ +52 |
| 3 | epic-16 §16-6 AC1 | the AC1 bullet | ⚠ R10 SUPERSEDED note: the admission order is **load → admit → start**, not the bullet's order; the gate list is §2b (no trust-tier/signature/region/vetting gate exists); `classify_spirit` is `:456` and returns **8 compiled-in classes**; the run block is `:4094-6047`; `enforce_vetted_upgrade_precondition` is `main.rs:134` with callers `:8840`/`operator_door.rs:709,799`. Original bullet kept **verbatim** below so its cites stay auditable |
| 4 | epic-16 §16-6 AC2 | `` a door-loaded Spirit unloads through 16-1's `unload` `` | ⚠ R10: **FALSE at HEAD** — `is_transition_allowed` (`control_block.rs:444-455`) has no `(Loaded, Unloaded)`; the SCB, pid and tokens leak and the name is burned. Requires the kernel arm (D-16-6-A) |
| 5 | epic-16 §16-6 AC3 | `` this story's docs say so while 17-1 is not `done` `` | ⚠ R10: already discharged twice (`README.md:282-284`, `ADR-062:161-165`); the obligation is that `load` adds **no new reachable surface** |
| 6 | epic-16 §16-6 *Closes · Δ* | `` kernel-Δ **0** `` | ⚠ R10: **kernel-Δ +1, ◆ FLAG-Winston**, seven re-pin artifacts; FR9 cite `:37` → **`:38`** |
| 7 | epic-16 Kernel-Δ paragraph | `` ZERO kernel-Δ for 16-1..16-4 `` … | ⚠ R10 appended: 16-6 takes the epic's **second** grant and the **third** post-16-0 re-pin; `RATIFIED_AT_EASING` (`recovery_lane_ceiling_rule.rs:268`) and the pin's line-position assert (`kernel_pin_content_hash_16_0.rs:846-851`) are the **sixth and seventh** artifacts 16-5's "five" did not count |
| 8 | epic-16 Kloc asks | `` the aggregate is 166542 … leaving **+4342** to hardfail `` | ⚠ R10: **167189** at `471608ea`, leaving **+3695**; `maos-bin` is **22996/22996 = ZERO**, and `grep 16-6 kloc.toml` has no match — nothing is reserved |
| 9 | epic-16 Dependencies line | `` `16-6` is kernel-Δ **0**, so it may land either side of 16-5 without a second re-pin; only 16-5 holds a grant `` | ⚠ R10: **FALSE** — 16-6 holds a grant and moves the pin a third time, so it lands **after** 16-5 and is the epic's genuine last commit |
| 10 | epic-16 exit block | 7 command lines | **NEW line 5b**, between lines 5 and 6 — `load` → `inspect` → **`unload` (Loaded→Unloaded, the grant)** → re-`load` (name not burned) → `start` (its first subject) → `unload` (Running→Unloaded), against the line-5 root and before line 6's `kill`. Second COMMAND edit in the epic's history; `bash -n` clean over all 8 lines; placeholder-free. ✅ **MEASURED**: `check-exit-commands` PASSES, 27→31 resolved, `load` **owed to `16-6` [ready-for-dev]** |
| 11 | epic-16 exit-block provenance list | `- Line 6 — …` | **NEW `- Line 5b …` bullet** naming all four HEAD reds (no `load` verb; `Loaded→Unloaded` refused; the burned name; `start` has no operator-creatable subject) and binding them to **16-6 AC1/AC2/AC3**, so `check-exit-commands`'s resolve-or-be-named leg binds |
| 12 | `requirements-inventory.md:437` | `` \| FR9 \| E1a (basic load/start/unload) + E5 … `` | ⚠ R10: FR9 is claimed by **Epic 16** — `start/pause/resume/unload` by `16-1`, **`load` by `16-6`** |
| 13 | `requirements-inventory.md:547` | `` \| FR9/FR13/FR16/FR24/FR51 verbs reaching a running Spirit (CANNED/PARTIAL) \| 16-1 \| `` | ⚠ R10: `16-1` **except FR9's `load`, which is `16-6`** |
| 14 | `sprint-status.yaml:252` 16-6 row | `` 16-6-…: backlog  # CREATED 2026-09-13 … (kernel-Δ 0). After 16-1, before 16-5. `` | `ready-for-dev` + a summary naming the `(Loaded, Unloaded)` grant, the inverted admission order, `start`'s first subject, the collection route and the exit line; prior comment kept after `PRIOR:` |
| 15 | `sprint-status.yaml:253` epic-16-retrospective row | the existing OWES list | ⚠ **cluster (a) is SPLIT**: the *loaded-but-never-started* half is **CLOSED AT ORIGIN by 16-6**; the *`on_unload`-hook-failure* half (and its false-success retry) **remains**, with §11 row 1's measurement attached. Appends §11 rows 2, 3 and 6 |

---

## 14. Open questions for the operator

✅ **ALL FIVE ARE CLOSED as of 2026-09-19 — Q1 GRANTED (four kernel files), Q4 = Shape A, Q5 = Shape A, Q2
confirmed in round 1 (R6), Q3 resolved at authoring.** The questions and their option sets are kept below
**unedited except for the ✅ ruling appended to each**, so the shape that was offered stays auditable beside the
shape that was chosen — the §15 discipline. Ruling record: **§17c**. **Nothing here blocks dev.**

**Q1 — the kernel grant (BLOCKING; RE-SCOPED by the 2026-09-18 round-table, §16a).** ⚠ **The grant is no
longer one line.** D-16-6-A recommends **Shape B** — `(Loaded, Unloaded)` in `is_transition_allowed` — **PLUS
the operator posture floor R3 escalated to a ship-blocker**: one `Option<Posture>` field beside the four floors
already in `OperatorPolicyConfig` and a `min()` at `security/mod.rs:361`. Together **+1 → ~+8 kernel lines**,
◆ FLAG-Winston, **TEN edits across EIGHT artifacts** (§15 V5). Same blast radius, same re-pin — **but it is not
the grant this question originally asked you for, and I am not re-scoping a FLAG-Winston silently.**
**Why R3 cannot wait for 17-1:** posture is the fifth axis of the leash and the only one of five with no
operator floor; `maosctl load` is the first thing that makes that absence reachable by a bearer holder, because
16-1's own `EndpointInUse` fast-fail removed the *"they could just exec `maos run`"* equivalence. Routing it
means shipping the corridor and filing the latch (§16a). The epic says this story is
`kernel-Δ 0`, so the recommendation **contradicts a ratified epic line** and needs your grant.
The case for B: FR9 names five verbs and Shape A collapses two of them; `maosctl start` has had **no
operator-creatable subject since 16-1 shipped it** (§3) and only B gives it one; B closes an
`epic-16-retrospective` row **at
origin** (rule 10(a)) and repairs an *existing* leak (`main.rs:5788`) that two prior stories documented and
neither could fix; and Shape C ships a pid/token/SCB leak and a burned name as a feature.
The cost, honestly: the epic's second kernel grant, the third post-16-0 re-pin, ten edits across eight
artifacts (including an existing kernel unit test that asserts the opposite, §15 V1), and 16-6 becomes the
epic's genuine last commit instead of landing either side of 16-5.
⚠ **And the honest counter-argument, which §15 V2 forced into the open:** a maos-bin force-unload IS available
at zero kernel-Δ — `scbs()` is `pub` and two production paths already bypass `transition`. Shape B is chosen on
**charter**, not impossibility. That is a weaker argument than the one this story originally made, and you
should weigh it knowing that.
**If you decline**, AC2 changes to Shape A (load-and-start), AC3 is withdrawn, FR9 is claimed at 4/5 with the
`start` gap stated in writing, and the `(Loaded, Unloaded)` row returns to `epic-16-retrospective` whole.
⚠ **RE-SCOPED A SECOND TIME by §17 R10.** The clamp is **two** `min()`s, not one — `security/mod.rs:357-366`
stores `current` and `allowed_max` both verbatim, and clamping only `allowed_max` leaves orchestrator and butler
running **above their own ceiling**. The grant is therefore **three kernel-core files**
(`scheduler/control_block.rs`, `capability/cap_policy/mod.rs`, `security/mod.rs`), `+10…+16` kernel lines,
**twelve** re-pin edits, **three** per-file digests. ✅ One thing got *cheaper*: the floor is forced to be
`Option<Posture>` (because `Posture` derives no `Default` while `OperatorPolicyConfig` derives `Default`), so it
defaults `None` and **`maos run` behaviour is provably unchanged** — the clamp cannot regress a shipped Spirit
unless an operator sets a floor.
✅ **RATIFIED by the operator 2026-09-19 — GRANTED as re-scoped, and WIDENED by Q4.** With Q4 = Shape A the
grant is **four** kernel-core files (`scheduler/control_block.rs`, `capability/cap_policy/mod.rs`,
`security/mod.rs`, `scheduler/scheduler_loop.rs`), `+16…+26` estimated kernel lines, **thirteen** re-pin edits
and **four** per-file digests. See frontmatter `kernel_grant` for the enumerated edit list and §17c for the
ruling record. Q1 is CLOSED; T3 is unblocked.

**Q4 — the `on_unload` receipt gap the arm makes reachable (BLOCKING; NEW in round 2, §17 R7).** Adding
`(Loaded, Unloaded)` routes a door-loaded, never-started Spirit into `fire_on_unload`
(`scheduler_loop.rs:501`) and the fallible `check_hook_outcome?` (`:502`) — and because the state already
flipped at `:483`, a hook panic or budget overrun strands an `Unloaded` SCB **in the map with no NFR-Rel-11
receipt**, after which the retry returns `Ok(())` and still writes none. §11 row 1 said this had no `load`
trigger; that is false. Three shapes:
**(A) Fix it here** — move `terminate_spirit`/revoke/drain/remove **ahead of** the hook, or demote the `?` to a
recorded non-fatal outcome, so cleanup always runs. Correct, closes the whole of `epic-16-retrospective`
cluster (a) rather than half, and makes AC2's *"a load that succeeds can always be undone"* true without an
asterisk. Cost: a **fourth** kernel-core file (`scheduler/scheduler_loop.rs`), a fourth pinned digest, and a
behaviour change on the `maos run` shutdown sweep that 16-2/16-3 already ratified.
**(B) Ship the arm, state the hole, keep the row** — AC2(g) captures the falsifier **red** (obligation (aa)),
the residual stays on `epic-16-retrospective` with its measurement attached, and AC2 says *"undoable except on
hook failure"* in writing. Cheapest, honest, and leaves a receipt gap reachable by an operator verb.
**(C) Refuse the load when the class enables a fallible `on_unload`** — no kernel-Δ, but measurement says it
buys nothing: no first-party compiled-in class overrides `on_unload` at all (the only non-test override is
`WorkerSpirit`, `supervision.rs:1672-1675`, and it returns `()`), so the failure modes are `Panicked` and
`BudgetExceeded` on the **default no-op hook** — a gate on the manifest cannot see them.
**Recommendation: (A).** The story's own charter argument for D-16-6-A — *the verb that creates a state owns the
state's exit* — applies with equal force to the exit that silently produces no receipt. (B) is the defensible
fallback and needs no grant beyond Q1.
✅ **RATIFIED 2026-09-19: Shape A — fix it here.** The cleanup-ordering repair ships in AC2(g);
`scheduler/scheduler_loop.rs` becomes the fourth kernel file and the fourth pinned digest;
`epic-16-retrospective` cluster (a) closes **WHOLE** and §11 row 1 leaves the cut table. Obligation (aa) still
captures the falsifier RED before the repair — a receipt gap never seen open is not a closed one.

**Q5 — how much of the admission path is actually unified (NON-BLOCKING but it decides what AC1 may CLAIM;
NEW in round 2, §17 R14).** AC1 promises *"No second admission implementation."* Measured, re-pointing the
standalone arm alone leaves **two**: shell (compiled-in manifest, hard-coded `HelloSpirit`, no `classify_spirit`,
no provenance) and topology (provenance **after** admit at `main.rs:4757`, vetter event on **grant only** at
`:4749` ⇒ **no vetter TL row on an admission rejection**). Topology **is** extractable — its loop
(`main.rs:4589`) handles one Spirit per iteration with every section locally owned (`:4551-4595`); the blocker
is effort, not types.
**(A) Re-point topology too** — one implementation for both real file-manifest paths, the §2c ordering
divergence **resolved instead of recorded**, and the missing vetter-rejection TL row fixed. Cost: `maos-bin` is
at ZERO headroom and the measured raise grows; topology is a shipped path, so its re-sequencing needs its own
proven-red.
**(B) Standalone only, and AC1 stops claiming what it has not done** — AC1 says *"one extracted function shared
by the door and the standalone arm"*, and the shell and topology copies are named as a §11 cut line with
`epic-16-retrospective` as owner. Cheapest, and it is the honest form of the current scope.
**Recommendation: (B) for scope, with one carve-out: the topology vetter-rejection TL row is an AUDIT GAP, not a
refactor** — a rejected admission that writes no vetter event is unauditable, and that is a two-line fix in the
`map_err` at `main.rs:4746-4748`. Take it here; leave the re-sequencing to the retrospective.
✅ **RATIFIED 2026-09-19: Shape A — re-point topology too.** One extracted function serves the door, the
standalone arm and the topology arm; §2c's ordering divergence is **resolved** and §11 row 2 leaves the cut
table; topology's `emit_vetter_key_event` fires on rejection as well as grant. Shell stays out of scope,
declared by name. The room's (B) recommendation is **overruled on the same criterion that overruled the 16-5
split** — long-term correctness over budget — and the budget consequence is recorded in AC1 and §10: the
`maos-bin` collapse is two call sites, `+420` is a floor not a ceiling, and the re-point of a shipped path
carries its own proven-red.

**Q2 — the ◆ mark.** The epic marks only 16-1 and 16-5 ◆ (non-degradable §A6 net). This story acquires both
properties that earned those marks — a kernel-core edit that moves the content-hash pin, and a new mutating
verb on the operator door. The frontmatter asserts ◆; **confirm**, since it binds the review net and the model
tier.

**Q3 — exit line 5b (low stakes, but it is a COMMAND edit, and it is ALREADY APPLIED).** D-16-6-F adds a line to the epic's hermetic block
so rule 1 stops being vacuous for this story (§6). It is only the second COMMAND edit in the epic's history.
The alternative is the named CI job alone (AC6), which is necessary either way but is not the control rule 1
names. ⚠ T0 must confirm the chosen class's `on_load`/`on_start` hooks are inference-free under the line-5
replay cassette, or the line hangs. ✅ **Resolved at authoring: `reviewer` is the chosen class and it is
already hard-coded into the applied line 5b.** Verified: `spirits/reviewer/manifest.toml` exists,
`class.name = "reviewer"` is in `classify_spirit` (`main.rs:462`) **and** in `successor_factory` (`:3137`), so
it dodges the researcher gap of §5. ⚠ **Note for the dev:** `reviewer` is `trust_tier = "local"` with
`[posture] default/allowed_max = assistive`, and neither the butler boot-loud guard (`main.rs:5206`) nor the
mira scalar-port guard (`:5700`) applies to it — so **two of §2b's eleven gates are unreachable on the exit
line, and AC1's "one refusal vector per gate" cannot be discharged by line 5b alone.** The per-gate vectors
belong in `maosctl_load_16_6.rs`.

---

## 15. Validation round 2026-09-18 — six re-rulings against this story's own headlines

An independent validator in a fresh context re-derived every citation and attacked the three §-headlines.
**Three of them were FALSE.** Recorded here rather than silently patched, because a preflight that quietly
corrects itself teaches the next story nothing.

| # | This story claimed | Measured | Applied at |
|---|---|---|---|
| **V1** | *"Zero test coverage of the exact case … the falsifier AC2 needs has never existed"* | **FALSE.** `fn loaded_to_unloaded_rejected` (`control_block.rs:567-573`) asserts the CURRENT refusal, so AC2 **reds `cargo test -p maos-kernel-core`**. It is inline `#[cfg(test)]` in a ZERO-headroom crate ⇒ charged by `kloc-check` | §1; frontmatter artifact (8); AC5; §10 kernel row `+1`→`+3…+6`; T3; obligation (w) |
| **V2** | *"There is no force-unload, no purge, no admin escape hatch anywhere in the tree"* and *"the SCB LEAKS PERMANENTLY"* | **FALSE — true of `SpiritSchedulerAdapter::unload` ONLY.** `handle_crash` discards the transition `Err` (`crash_detector.rs:120-124`) and still revokes, receipts and **`map.remove`s** (`:127`/`:130`/`:191-194`); cold-swap's rollback removes outright (`upgrade.rs:200-203`); and `scbs()` is `pub` (`scheduler_loop.rs:148`), so maos-bin could do it in three lines at zero kernel-Δ | §1 scope table; **D-16-6-A Shape D re-argued on three grounds instead of on impossibility**; §11 row 3; obligation (m2) |
| **V3** | *"`maosctl start` has no reachable subject at HEAD"* | **FALSE.** The door binds at `main.rs:3362`, **before** the run block at `:4094`, and a failed Worker `start` (`supervision.rs:1334`→`:1339`) leaves a **durable** `Loaded` SCB under a `validate_id`-passing id inside a serving root. The surviving claim is **operator-creatable**, not *existent* | §3 heading and body; AC3 proven-red; obligation (i); **and the epic's line-5b provenance bullet (d), which had already shipped the false form** |
| **V4** | *"`successor_factory` … for all 8 classes"* | **FALSE — SEVEN + `smoke-spirit`; there is no `"researcher"` arm** (`main.rs:3123-3198`, match `:3130`), while `classify_spirit` maps it (`:459`) and the standalone arm loads one (`:5645`). Naive reuse would make `maosctl load` unable to admit a class `maos run` can | §5; Dev notes; D-16-6-B; T2 |
| **V5** | *"the re-pin touches SEVEN artifacts"* | **Undercounted — TEN edits across EIGHT artifacts.** `kernel_pin_content_hash_16_0.rs` holds **four** `24628` literals (`:802`, `:820`, `:829`, `:840`), and V1 adds the test. ⚠ This is the same undercount the story accuses 16-5's *"five"* of | frontmatter `kernel_grant`; AC5; T3 |
| **V6** | *"three verbatim copies"* collapse; *"the net could approach zero or go negative"* | **Overstated, in the expensive direction against a ZERO-headroom crate.** The shell triple parses a **compiled-in** manifest (`main.rs:3741`) with no file read, no `classify_spirit`, no provenance gate; and AC1/T2 re-point **the standalone arm only** — one call site, not three | §10 `maos-bin` row and its note |

Smaller corrections applied in the same pass: `check-exit-commands` owes **4** at HEAD, not 3 (`maos-spirit`
was omitted); **`Cargo.toml:60` is `default-members = []`**, so the gate needs `cargo build -p maos-bin -p
maos-cli --bins` first or it reports ~18 spurious findings; `scheduler.load` sites are **20**, not 21; AC6 now
describes the six-verb line that actually landed; and fifteen `file:line` cites were re-pinned
(`LONG_ROUTE_BUDGET` `:227`, route match `:980`, bearer `:901-910`, 413 `:765-770`, `route_budget` `:367-375`,
`OperatorOutcome` `:384-399`, CAS `:1382-1390`, `BinPrivateOpsImpl` `:3330-3338` as **arg 18 of 19**,
`current_state` `:1461`, cold-swap refusal `:632-643`, `Barrier` `:1044`, `successor_factory` `:3123-3198`).

**Two obligations this round adds:**
**(w)** Delete the `(Loaded, Unloaded)` arm after landing ⇒ confirm `fn loaded_to_unloaded_rejected` goes
**green again**, proving the test and the arm are the same fact and that AC2 did not simply delete its own
oracle. **(x)** Load a **`researcher`** manifest over the door ⇒ it must admit; if the extraction was built on
`successor_factory` unextended, it will fail `UpgradeError::SuccessorFactory` while `maos run` admits the same
manifest — the capability regression V4 names.

---

## 16. Round-table 2026-09-18 — six rulings, and one ship-blocker the story did not have

Convened as a preflight on the authored story under the standing criterion: **spec fidelity + long-term
correctness.** Present: Mary, Paige, John, Sally, Winston, Amelia, Murat, with Grumbal, Vex and Splinter
summoned. Two rulings were settled by measurement taken at the table; one **escalated a wording fix into a
ship-blocker**.

| # | Fork | Ruling | Applied at |
|---|---|---|---|
| **R1** | §15 V2 destroyed Shape B's *impossibility* argument; the story fell back to "charter" | **Shape B stands, re-argued on INVARIANT OWNERSHIP.** Winston: the two existing bypasses **disagree about receipts** — `handle_crash` revokes, receipts and journals (`crash_detector.rs:127`/`:130`/`:181-189`); cold-swap's rollback writes **nothing** (`upgrade.rs:200-203`). A maos-bin force-unload must therefore *pick one*, deciding **NFR-Rel-11's meaning in an adapter**. Charter was the weak form of this. | **D-16-6-A** rejected-column; §1 |
| **R2** | Sally: *"I load it, I get distracted, I go to lunch — what's watching it?"* | **Nothing is** — the DRR picker selects only `Running`, the watchdog spawns in the serving tail. But the fix is **observability, not supervision**: `GET /v1/daemon` returns `spirit_ids` **only** (`maos-control/src/lib.rs:985-995`), so a loaded-and-forgotten Spirit is **indistinguishable from a working one** on the only surface that enumerates the daemon. One field. **Constraint on AC1, not a new AC** (John, upheld). | **AC1** new clause; obligation (y) |
| **R3** | ⛔ **Posture is UNCLAMPED — `load` DOES add privilege, and the story says it doesn't** | **SHIP-BLOCKER. The clamp ships here.** See §16a. | **AC1** new gate + vector; §16a; §14 Q1 re-scoped |
| **R4** | May a story overturn the epic's ratified `kernel-Δ 0`? (Murat's own rulings-rot rule) | **Yes, narrowly: a story may CORRECT A MEASUREMENT; it may not RE-RULE A DECISION.** D3 was a ruling — someone weighed options. `kernel-Δ 0` is a **measurement claim**, and it is false. Murat's rot-rule survives intact. | **D-16-6-A**; spec_alignment |
| **R5** | AC5's ten-edit list is a premise, not a control (Yui's rule, via Murat) | **One vector, ten observations, ONE verdict.** Splinter trimmed it: ten named assertions that all fail for the same reason is *"a checklist wearing a lab coat."* The control must be unable to pass while the list is short. | **AC5**; obligation (w) |
| **R6** | Q2, the ◆ mark | **Confirmed, uncontested** — kernel edit + pin move + new mutating door verb. | frontmatter `review` |

### 16a. R3 — the escalation chain, measured at the table

The story inherits the epic's AC3 claim that *"a bearer holder can already exec `maos run`, so `load` adds no
privilege."* **Measured, that is FALSE**, and **16-1's own fast-fail is what makes it false**: a second root
binding the same endpoint dies `EndpointInUse` + exit 78 (`main.rs:3375-3391`), so `maosctl load` is **the only
runtime path a manifest has into the live root.**

Five steps, all shipped verbs, no new code, inside the eight-class bound of §5:

1. Read `<home>/control.json` — a bare Worker runs as the operator uid (`README.md:282-284`, no `env_clear` at
   `runtime.rs:453`, and `credential_posture_2c.rs:299` forbids adding one until 17-1).
2. Write a manifest naming a **compiled-in** class (`reviewer`) with `[posture] allowed_max = autonomous-with-halt`.
3. `POST /v1/spirits`.
4. `admit_spirit` stores the ceiling **VERBATIM** — `security/mod.rs:359-366`,
   `allowed_max: posture_section.allowed_max`, **no clamp of any kind**.
5. `maosctl posture <id> --shift autonomous-with-halt` — 16-1's own verb — passes its ceiling check at
   `cap_policy/mod.rs:255` (`if new_posture > state.allowed_max`), because the ceiling is *the attacker's own
   number*.

**The asymmetry is the proof.** `OperatorPolicyConfig` (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:22-43`)
carries **four** operator floors: `global_sandbox_floor` (*"applied to ALL Spirits regardless of manifest"*,
`:26`), `spirit_tier_floor` (`:24`), `resource_cap_floor` (`:30`), and two subtract-only capability deny sets
(`:38`, `:42`) whose comment states the design principle outright — *"without letting PDP permits raise the
manifest ceiling."* Resource caps are resolved strictest-of at `security/mod.rs:418-426`.

**Posture is the fifth axis of the leash and the only one with no floor.**

⇒ **The clamp ships in 16-6, as a new gate under AC1** — not a new AC (John), because AC1 already promises
*"every gate `maos run` applies, applies identically", and this is a gate that exists on **neither** path.
A missing gate cannot have a refusal vector written for it (Murat). Shape: one `Option<Posture>` field beside
the four existing floors + a `min()` at `security/mod.rs:361`. **It is a consistency repair, not a
governance subsystem** — Splinter withdrew his over-building objection on that basis.
⚠ **The admission record must carry BOTH the requested and the effective ceiling** (Sally): a clamped
admission that looks identical to an unclamped one is the claim-standing-in-for-a-control shape again.
⚠ **Budget:** the FLAG-Winston grant grows **+1 → ~+8** kernel lines. Same blast radius, same re-pin, same ten
edits — but it is **not the grant §14 Q1 originally described**, and Q1 is re-scoped accordingly.

**Falsifier (red at HEAD in one run):** operator floor `assistive`; manifest declares
`allowed_max = autonomous-with-halt`; `POST /v1/spirits` ⇒ stored ceiling must be **`assistive`**; then
`maosctl posture --shift autonomous-with-halt` ⇒ **`AboveCeiling`**. At HEAD the shift **succeeds**.

**Two obligations this round adds:**
**(y)** `maosctl load` without `start`, then `GET /v1/daemon` ⇒ the row must report `Loaded`; confirm that at
HEAD the endpoint returns `spirit_ids` only and the Spirit is indistinguishable from a Running one.
**(z)** Run §16a's five-step chain **end to end at HEAD** and confirm the posture shift **succeeds** — the
escalation must be seen working before it is closed. A clamp never seen bypassed is not a control.

---

## 17. Preflight round 2 — 2026-09-19 — fourteen findings; §11's cut line 1 is withdrawn and three of AC1's own refusal vectors are unwritable

Convened on the same standing criterion: **spec fidelity + long-term correctness**. Six independent scouts read
the tree before anyone spoke. **Round 1's own R3 fix is the wrong line, §11 cut line 1 has a `load` trigger
after all, three of AC1's eleven refusal vectors cannot be written at HEAD, and AC4's compile-safety claim is
false.** Recorded here rather than silently patched — the §15 discipline.

| # | Finding | Measured | Applied at |
|---|---|---|---|
| **R7** | ⛔ **§11 cut line 1 has a `load` trigger — the arm makes the `on_unload` receipt gap reachable from the exact verb pair this story ships** | `SpiritSchedulerAdapter::unload` (`scheduler_loop.rs:471-523`) flips the state at `:483` **before** the fallible `check_hook_outcome?` at `:502`; receipt/revoke/drain/remove are all after it. Nothing between `:483` and `:502` is conditioned on the prior state, and `kernel_invocation_allowed` (`maos-spirit-abi/src/lifecycle.rs:510-515`) returns **`true` when `enabled_hooks` is empty**, so a default-manifest door load **does** dispatch `on_unload`. A `Panicked`/`BudgetExceeded` outcome (`hook_dispatch.rs:573-588`) leaves an `Unloaded` SCB still in the map; the operator's retry hits the `:479-481` early return and gets **`Ok(())` with no receipt**. §11 row 1's *"has no `load` trigger"* is **FALSE** | §11 row 1 **withdrawn**; AC2 new vector (g); §14 **Q4**; obligation (aa) |
| **R8** | §2's sandbox-downgrade argument for D-16-6-B is **FALSE** | The single `SpiritSchedulerAdapter::new` in maos-bin passes **`security_manager: None`** (`main.rs:2793`, comment *"constructed per-one-shot arm at v0.3-β"*), so the kernel's internal `admit_spirit` (`scheduler_loop.rs:276-322`) **never runs in the daemon** — it takes the `else` branch at `:323-334` and emits a bare `lifecycle.admit` frame. A door `load` calling the kernel directly would therefore admit **not at T2 but not at all**. D-16-6-B's ruling survives; its stated reason does not. (There is also no double-admission at HEAD, so §2's worry about it is moot) | §2 ⚠; D-16-6-B rejected-column; obligation (g) **re-aimed** |
| **R9** | ⛔ **THREE of AC1's eleven refusal vectors cannot be written — the manifest's `[sandbox] tier` is parsed and then discarded** | `effective_sandbox_tier` (`cap_policy/mod.rs:201-224`) seeds the manifest leg from **`manifest_scopes[pid].declared_tier`**, which is **absent on first admission** ⇒ `SandboxTier::DEFAULT_FLOOR`; `admit_spirit` then writes `declared_tier: effective` back (`security/mod.rs:349-355`). Its `manifest: &SandboxConfig` parameter is used **only** for `image_pin` (`:403`). A workspace grep confirms **no reader** of `SandboxConfig.tier` on the admission path. With `global_sandbox_floor` defaulting to `T2` and `spirit_tier_floor`/`trust_tier_floor` empty in production, `effective` is **always T2** for the eight compiled-in classes ⇒ `SandboxTierUnsupported` for **T4** (`:410-412`) and **T1** (`:414-417`) and **`T3AdmissionFailed`** (`:385-402`) are all three **unreachable from a manifest**. This is Vex's 12.1-R2 *accept-and-ignore* shape: a declared control that is parsed and unenforced | **AC1** vector list corrected; D-16-6-E `route_budget` justification corrected; §11 new row 7; obligation (ab) |
| **R10** | ⛔ **§16a's clamp is the WRONG LINE — `allowed_max`-only inverts the ceiling invariant** | `security/mod.rs:357-366` stores **both** `current: posture_section.default` **and** `allowed_max: posture_section.allowed_max` verbatim, and `RawPostureSection::validate` (`maos-manifest/src/manifest.rs:687-701`) enforces only `allowed_max >= default`. A `min()` at `:361` alone yields, for orchestrator (`default = autonomous-with-halt`) under a `assistive` floor, `current=AutonomousWithHalt, allowed_max=Assistive` — the Spirit **runs above its own ceiling** and the clamp clamps nothing. **Both fields must be `min()`-ed.** Good news, measured: `Posture` (`manifest.rs:643-653`) derives `Ord` least-privilege-first (`Cautious < Assistive < AutonomousWithHalt < Autonomous`) so `min()` is a real clamp; and `OperatorPolicyConfig` derives `Default` while `Posture` derives **no** `Default`, forcing `Option<Posture>` ⇒ `None` ⇒ **`maos run` behaviour provably unchanged** | **AC1** R3 clause; §16a; §14 Q1; obligation (ac) |
| **R11** | **AC4's compile-safety claim is FALSE — `route_budget` is not total, and there are THREE exhaustive matches, not four** | `route_budget` (`maos-control/src/lib.rs:367-381`) ends `_ => DEFAULT_ROUTE_BUDGET`. The exhaustive matches are **`spirit_key` (`:345-364`), `DoorInner::dispatch` (`operator_door.rs:329-420`), `command_verb` (`operator_door.rs:1526-1544`)** — the enum derives only `Debug, Clone, PartialEq, Eq` (`:270`), there is no `Display`/`Serialize`/metrics impl, and `spirit_command` (`:1222`) matches `&str` not the enum. So `Load` **silently inherits `DEFAULT_ROUTE_BUDGET` = 10 s** (`:227`) and the compiler never says so. ⚠ And R9 removes the story's stated reason for `LONG_ROUTE_BUDGET` (*"admission does T3 image-lock verification"* — it never does); the surviving reason is the `on_load`/`on_start` hook budgets inside the triple | **AC4**; D-16-6-E; obligation (l) **promoted from optional to binding** |
| **R12** | **D-16-6-D's lock lands after the point of no return, and its wait is unbounded** | The per-Spirit lock is taken in the **port**, `run_command` (`operator_door.rs:1323-1345`): `lock_wait = deadline − 250 ms`, `command.spirit_key()`, `timeout(lock_wait, lock.lock())` — **then** the `QUEUED → STARTED` CAS (`:1349-1357`), **then** `dispatch` (`:1370`). Because `Load`'s `spirit_key()` is `None`, no port lock is taken and D-16-6-D's handler lock runs *inside* `dispatch`, i.e. **after** the CAS: (a) a contended `Load` can never lose the withdraw CAS, so it answers **`handler_still_running`** where every other verb answers `spirit_busy`; (b) a bare `.lock().await` is **not bounded by `lock_wait`**, on a task deliberately never cancelled (`:1407`). ✅ Separately CONFIRMED: the lock is port-level, so **`shell_host.rs:171`'s path is covered** | **D-16-6-D** amended; **AC4**; obligation (ad) |
| **R13** | **An unlisted prerequisite: the door seam cannot admit or register a pid** | `BinPrivateOpsImpl` (`main.rs:8768-8776`) holds `memory`, `capability`, `transparency_log`, `audit_db_path`, `memory_db_path`, `shared_journal`, `upgrade_orchestrator` — **no `security`, no `pid_by_spirit_id`** — and `trait BinPrivateOps` (`operator_door.rs:54-107`) declares five methods, none able to load or admit. The standalone arm inserts into `pid_by_spirit_id` at `main.rs:5819`. So `BinPrivateOps`/`BinPrivateOpsImpl` must be **widened with `security` and `pid_by_spirit_id`**, which no AC or task says | **AC1**; **T4**; Dev notes |
| **R14** | **AC1's *"No second admission implementation"* is FALSE as T2 is scoped — and topology IS extractable** | After re-pointing the standalone arm only, two independent implementations survive: **shell** (`:3740-3856` — parses the compiled-in `maos_spirit_hello::MANIFEST_TOML` `:3741`, hard-codes `HelloSpirit` and the id `"hello-spirit"`, never calls `classify_spirit`, no provenance gate, no `pid_by_spirit_id` insert) and **topology** (`:4600-4790` — provenance **after** admit `:4757` vs standalone's **before** `:5135`, and `emit_vetter_key_event` on **grant only** `:4749`, so its `map_err` at `:4746-4748` leaves **no vetter TL row on rejection**, unlike shell `:3832` and standalone `:5779`). Extractability measured: the topology loop (`:4589`) is **one spirit per iteration** with every section locally owned (`:4551-4595`) — no `Vec<Spirit>`, no lifetime blocker; only Mira's `ButlerOrchestratorAdapter` (`:4664-4696`) needs the pid returned. **The blocker is effort, not types** | §14 **Q5**; **AC1** claim restated; §11 row 2 re-aimed |
| **R15** | **AC5 is UNDERSTATED and the edit count is 12, not 10** | R3 edits `capability/cap_policy/mod.rs` **and** `security/mod.rs`, whose digests are separate rows at `kernel-core-baseline.toml:548` and `:614`, beside `scheduler/control_block.rs` at `:601` ⇒ **THREE digests change by name**, not one; a dev satisfying AC5 literally passes while two kernel files silently change — the D-16-0-F failure mode the per-file map exists to catch. `PINNED_ENTRIES` stays **98**. ⚠ **NEW HOLE nobody counted:** every prior re-pin **prepended** a multi-line `# HISTORY:` block above `src_lines` (16-5's is `:496-504`, nine lines); doing that again pushes `src_lines` off 505 and reds `kernel_pin_content_hash_16_0.rs:842-851`. The 16-6 HISTORY row must be **line-count-neutral** | **AC5**; frontmatter `kernel_grant`; **T3**; obligation (ae) |
| **R16** | **The `spirits`-map write census is SIX, not three — obligation (m2) is wrong** | Writes: `scheduler_loop.rs:348` insert, `:520` remove; `upgrade.rs:198` insert, `:203` remove, **`:208` re-insert**; **`crash_detector.rs:193` `map.remove`**. `scbs()` is `pub` (`scheduler_loop.rs:148`) with ~30 external call sites across `main.rs`, `operator_door.rs`, `supervision.rs` and tests. And **`upgrade.rs:164` (cold-swap unloading the *predecessor*) is a caller the story never lists** — it changes behaviour when the predecessor was loaded-but-never-started | obligation **(m2) corrected**; §11 row 3 |
| **R17** | **The arm turns two production comments into lies and flips `report.failed` from over- to under-reporting** | `supervision.rs:1339-1342` (*"there is no `Loaded → Unloaded` transition, so it cannot be unloaded and gets no receipt"*) and `shell_host.rs:241-247` (*"this unload ALWAYS fails"*) both become **false** the moment the arm lands, as does the test comment at `shell_host_16_2.rs:545-547`. In `unload_all_loaded` (`supervision.rs:139-167`) a `Loaded` pid stops landing in `report.failed` — correct — but combined with R7 the **real** hook-failure now also disappears from it, and `finish_shell_session`'s `had_failures` short-circuit (`shell_host.rs:248-253`) stops skipping the admission-failure path. Mary's *read the build, not the prose* rule, applied to comments **this story creates** | **T3**; obligation (af) |
| **R18** | R2 is additive-safe **only as a new sibling key** | `GET /v1/daemon` (`maos-control/src/lib.rs:981-994`) returns `spirit_ids` as a bare `Vec<String>` (`DaemonStatusRow:436`), produced sorted at `operator_door.rs:1469-1476`. The only value assertion in the tree is `assert_eq!(daemon["spirit_ids"][0], "butler")` (`crates/maos-control/tests/post_surface_16_1.rs:1238`); `maos-cli` has **zero** consumers. So a new sibling key is safe, **reshaping `spirit_ids` into objects is not**. ⚠ `DaemonStatusRow` is **not `#[non_exhaustive]`** (`:430-439`) ⇒ a new field is a compile break at three struct literals (`operator_door.rs:1472`, `post_surface_16_1.rs:294`, `submit_and_wait_16_2.rs:49`) — compiler-caught, three sites | **AC1** R2 clause; obligation (y) sharpened |
| **R19** | `admission.rs` lands **inside an existing negative** | The `SCANNED_SOURCE_FILES` roster (`cohort_daemon_smoke_13_5c.rs:882`, **23 at HEAD, exact**) feeds the `manifest_scopes` negative at `:992-995`, and `admission.rs` is **not** whitelisted the way `enterprise_pdp_runtime.rs` is ⇒ **the extracted `load → admit → start` function must never touch `manifest_scopes`**. The directory-listing equality assert is at **`:971-985`** (the story says `:981-985`) | **AC6**; **T2**; Project Structure Notes |
| **R20** | The `successor_factory` researcher gap is wider than V4 said | A `researcher` admitted over the door can also never be **hot-upgraded** — it dies in `unsupported => UpgradeError::SuccessorFactory` (`main.rs:3186`). The round-trip test at `main.rs:13690-13718` asserts only `classify_spirit` totality, so **nothing guards factory/classify parity**. A dev note is not a control | **T2**; obligation (x) extended to a **parity test** |

### 17c. Operator rulings — 2026-09-19

| Q | Ruling | Consequence |
|---|---|---|
| **Q1** — the re-scoped FLAG-Winston grant | ✅ **GRANTED as re-scoped**, and widened by Q4 to **four** kernel-core files | `+16…+26` kernel lines, **thirteen** re-pin edits, **FOUR** per-file digests. T3 unblocked; frontmatter `kernel_grant` carries the enumerated list |
| **Q4** — the `on_unload` receipt gap (§17 R7) | ✅ **Shape A — fix it here** | AC2(g) ships the cleanup-ordering repair in `scheduler_loop.rs:471-523`; `epic-16-retrospective` cluster (a) closes **WHOLE**; **§11 row 1 leaves the cut table**; obligation (aa) still captures the falsifier RED first |
| **Q5** — how wide the extraction goes (§17 R14) | ✅ **Shape A — re-point topology too**, against the room's (B) recommendation | One function serves the door + standalone + topology; §2c's divergence **resolved**, not recorded; **§11 row 2 leaves the cut table**; topology's vetter event fires on rejection; shell declared out of scope by name; `maos-bin` collapse is two call sites so `+420` is a floor |

**The criterion that decided Q5 is the one that decided 16-5's sizing:** spec fidelity and long-term
correctness over budget. The room recommended the cheaper honest scope (B — narrow the claim, name the copies);
the operator took the wider one, which means AC1 gets to keep its original sentence — *"No second admission
implementation"* — as a **true** statement about both file-manifest paths rather than a narrowed one. Dana's
budget dissent is recorded; `maos-bin` at ZERO headroom is the real cost and T2 measures it twice.

⚠ **Q2** (the ◆ mark) was confirmed in round 1 (R6) and round 2 adds a **third** qualifying property — this
story now ships a **privilege clamp** — so the §A6 net gains a Security layer by name (frontmatter `review`).
**Q3** was resolved at authoring. **No open questions remain.**

### 17d. Obligations this round adds

**(aa)** Force an `on_unload` hook failure on a **door-loaded, never-started** Spirit (panic or exceed the hook
budget, `hook_dispatch.rs:573-588`) ⇒ confirm the SCB is `Unloaded`, **still in the map**, has a
`lifecycle.unload` TL row and **no `term-…` receipt**; then retry `maosctl unload` ⇒ confirm it returns **exit 0**
(`scheduler_loop.rs:479-481`) with still no receipt. That is R7's falsifier, and it must be seen red before any
fix is accepted. **(ab)** Write `[sandbox] tier = "T4"` in a door-loaded manifest ⇒ confirm admission **succeeds
at T2**, proving the declared tier is discarded (`cap_policy/mod.rs:207-211`) and that AC1 must not promise a
T1/T4/T3 refusal vector. **(ac)** With the posture floor set to `assistive`, load the **orchestrator** manifest
(`default = autonomous-with-halt`) ⇒ confirm the stored `PostureState` has `current == Assistive`, not
`AutonomousWithHalt`; then clamp `allowed_max` **only** and confirm `current > allowed_max` is observable — the
R10 inversion. **(ad)** Submit two `Load`s that contend on the same id and confirm the loser answers
**`spirit_busy`**, not `handler_still_running`, and that its wait is bounded by `lock_wait`
(`operator_door.rs:1323-1330`); delete the bound ⇒ confirm the wait goes unbounded. **(ae)** Prepend a
multi-line HISTORY block above `src_lines` ⇒ confirm `kernel_pin_content_hash_16_0.rs:842-851` reds on the line
position; then land the row **line-count-neutral** and confirm green. **(af)** After the arm lands, grep
`supervision.rs:1339-1342`, `shell_host.rs:241-247` and `shell_host_16_2.rs:545-547` ⇒ confirm each comment was
rewritten, not left asserting the pre-arm behaviour. **(ag)** Add `("admission.rs", …)` to
`SCANNED_SOURCE_FILES` and confirm the `manifest_scopes` negative at `cohort_daemon_smoke_13_5c.rs:992-995`
covers it — then touch `manifest_scopes` from `admission.rs` and confirm it reds. **(ah)** Enumerate the
`OperatorCommand` matches after adding `Load` ⇒ confirm **three** are exhaustive and that `route_budget`
(`maos-control/src/lib.rs:367-381`) has a `_` arm, so the `Load` budget arm is **uncompiled-checked** and
obligation (l) is the only control. **(ai)** Widen `BinPrivateOps` with `security` + `pid_by_spirit_id`, then
remove the `pid_by_spirit_id` insert ⇒ confirm a door-loaded Spirit becomes unaddressable by id on a surface
that resolves through it.

---

## 18. Preflight round 3 — 2026-09-19 — "is it truly ready?" — FOUR DAY-ONE BLOCKERS, two of them in round 2's own fix

Asked whether the story was genuinely ready, the room did what it did at 12.1 round 3 and 13.2 round 2: it ran
four scouts **against its own ratified resolution** instead of reaffirming it. **It was not ready.** The
standing lesson holds a third time on a third story: *readiness is not a claim you make, it is a graph you draw
and a build you run.*

| # | Blocker | Measured | Forced correction |
|---|---|---|---|
| **R21** | ⛔ **Q4 Shape A offered TWO mechanisms and one of them is BROKEN. Mechanism (y) — "demote the `?` at `scheduler_loop.rs:502`" — DOUBLE-CLEANS.** | `check_hook_outcome`'s `Panicked` arm (`scheduler_loop.rs:573-595`) `tokio::spawn`s `CrashDetector::handle_crash`, which performs a **full parallel teardown** of the same pid: transition `:120-124`, `revoke_all_for_pid` `:127`, `terminate_spirit(UnplannedCrash)` `:130`, `task.orphaned` `:163-170`, `enforce_disposition`, journal `Crash` + `map.remove` `:191-194`. Today the `?` is the only thing keeping the two teardowns apart. Demote it and on `Panicked` both run concurrently ⇒ either (i) `unload` wins and the FR50 disposition/orphan frames are lost **non-deterministically**, or (ii) `handle_crash` wins and you get **TWO** receipt pairs for one termination — one `planned_unload`, one `unplanned_crash` — a duplicate revoke, and a `Crash` journal row for a *planned* unload. ⚠ `Ok(Err(join_err))` also maps a **cancelled** hook to `Panicked` (`hook_dispatch.rs:657-664`), so the race is not panic-only | **Mechanism (y) is REFUSED. AC2(g) pins mechanism (x)** — cleanup ahead of the hook — **plus an explicit guard: `check_hook_outcome`'s `Panicked` arm must NOT spawn `handle_crash` when `hook_name == "on_unload"`**, because `unload` already owns revoke + `terminate_spirit` + `map.remove`. ⚠ The weaker guard *"spawn only if the pid is still in the map"* **does not work** — at `:502` the pid IS still in the map, so the race persists. This makes the `scheduler_loop.rs` edit larger than Q4 priced |
| **R22** | ⛔ **`terminate_spirit` is NOT idempotent — which is what turns R21's race into a durable audit defect rather than a benign retry.** And mechanism (x) has two costs nobody priced | `HaltId::new(format!("term-{spirit_id}-{spirit_pid}-{timestamp_ns}"))` (`halt/termination.rs:44-45`) takes `timestamp_ns` from `SystemTime::now()` (`:34-37`); `boot_nonce` reaches only the receipt **body** (`:65`, `:101`), never the id. Two calls mint two different `HaltId`s and write **four** `EpistemicHalt` TL frames (`:53-60`, `:74-81`) with mismatched `kind_str`. No dedup exists. ⚠ **(x)-a:** with `spirits.remove` moved ahead, a panicking `on_unload`'s spawned `handle_crash` hits its Step-1 lookup (`crash_detector.rs:78-84`) and returns `NotLoaded`, swallowed by the `let _ =` at `scheduler_loop.rs:581` ⇒ **`task.orphaned` and `enforce_disposition` are silently lost**, which today still fire. ⚠ **(x)-b:** `terminate_spirit(PlannedUnload)` writes `HaltState::Terminated` (`termination.rs:63`, `:99`) **before the hook ever signals the child** (`WorkerSpirit::on_unload` → `begin_stop` → `execute`'s `Signal` arm → `strategy.signal_kill`, `supervision.rs:1670-1675`, `:700-707`, `:740-800`) ⇒ a receipt attesting termination of a **live** process | **AC2(g) must additionally (a) perform `task.orphaned` + `enforce_disposition` INLINE in the `on_unload`-failure path, since (x) takes them away from `handle_crash`; and (b) resolve the receipt-ordering fork — see §18a.** ✅ Otherwise (x) is mechanically SAFE: the hook ctx is `Ctx::for_rust_inproc_hook(CapabilityHandle(0), MailboxHandle(0))` built inside the `spawn_blocking` closure (`hook_dispatch.rs:594-602`) with only `spirit_pid` and the `spirits` map, so **nothing a hook can reach is destroyed by an early revoke/drain/remove**, and `fire_payload_hook` dispatches from the `&scb` Arc it was handed (`:580-601`), never from the map. ⚠ Callback to 13.1: `HookDispatcher::build_kernel_ctx` (`hook_dispatch.rs:146-177`) — which *would* wire capability/halt/TL — **has exactly one occurrence in the crate: its own definition. It is dead.** |
| **R23** | ⛔ **The posture clamp REDS A SHIPPED TEST, and the story's own "requested + effective ceiling" requirement has NOWHERE TO LIVE.** | `one_daemon_one_door_16_1.rs:390-403` reads `root.field("butler","posture")`, shifts butler to `cautious`, then `assert_ne!(before, after, "the shift must MOVE the posture the policy table serves")`. Butler's manifest is `default = "assistive"` (`spirits/butler/manifest.toml:43`), so `before == "Assistive"` today; under a `Cautious` floor `admit_spirit` stores `min(Cautious, Assistive) = Cautious` ⇒ `before == after` ⇒ **`assert_ne!` fails in this story's own end-to-end door fixture.** Separately the clamp is a **SILENT LOWERING**: `security/mod.rs:357-366` gains no error and no warn, and the Load journal row at `:437-444` is a `LifecycleEntry` (`i10.rs:121-129`) with **no posture field**. The only posture surface anywhere is `GET /v1/spirits/{id}`'s five fields (`maos-control/src/lib.rs:1024-1029`), where `posture` is `format!("{:?}", state.current)` (`operator_door.rs:1452-1456`) — the **requested** current and **both** ceilings are unobservable; `maosctl spirit inspect` is sandbox-only (`maos-cli/src/cli.rs:859-864`) | **(a)** The floor must be **null in every fixture that reuses the 16-1 daemon**, and the clamp's own vectors get their own root — stated, not assumed. **(b)** §16a's *"the admission record must carry BOTH the requested and the effective ceiling"* requires a **SIXTH key** on `GET /v1/spirits/{id}` plus a posture field on the admission record: `SpiritStatusRow` (`maos-control/src/lib.rs:418-428`), the JSON literal (`:1024-1029`) and `spirit_status`'s construction (`operator_door.rs:1462-1468`). ✅ Additive-safe — no `as_object().len()` pin on this route — but **round 2 budgeted none of it, and without it the AC is UNVERIFIABLE.** ✅ CLEAR, separately: the butler/mira boot-loud guards are **posture-independent** — `needs_port` = `requires_epistemic_halt_port` (`main.rs:564-580`) reads only `epistemic_policy.rules`, so no clamp can flip either guard; `posture_section.allowed_max` appears only inside butler's error *format string* (`:5208-5215`) |
| **R24** | ⛔ **The CI job AC6 tells the dev to copy VERBATIM never builds `maosctl` — a copied 16-6 job panics on a cold runner.** | `one-daemon-one-door` (`.github/workflows/discipline.yml:3643-3655`) has exactly one build step: `cargo build --locked -p worker --bin worker-cli-fixture`. But `one_daemon_one_door_16_1.rs:54-68` resolves `maosctl` beside `CARGO_BIN_EXE_maos` and **hard-asserts `path.is_file()`** with the message *"run `cargo build -p maos-cli` first"*. `maos-cli` is a plain **`[dependencies]`** entry of maos-bin (`crates/maos-bin/Cargo.toml:44`, section header `:39`), so cargo builds its **lib**, never its `[[bin]] maosctl` (`crates/maos-cli/Cargo.toml:10-12`). The existing job is green only because `Swatinem/rust-cache@v2` restores a `target/` that some other job populated — a gate passing for a reason unrelated to what it asserts | **AC6's job MUST add `cargo build --locked -p maos-cli --bin maosctl` beside the `worker-cli-fixture` step.** ⚠ And the 16-1 job has the same latent defect — report it to `epic-16-retrospective` rather than copying it. ✅ CLEAR on the axis the round was sent to check: the enrollment **does** block per-commit. `discipline.yml:3-7` is `push`/`pull_request` on `main` with no ref guard; `v1-0-ship-gate` carries only `if: always()` (`:3729`) and exits 1 on any failed/skipped/cancelled need (`:3780-3782`); and **`aggregate.needs` contains `v1-0-ship-gate` (`:3928`)** with its own `exit 1` (`:4155-4158`). The Epic-11 *"ran but returned Ok"* decay does **not** apply. ⚠ RISK only: `check_ship_gate_completeness` iterates `EXPECTED_GATES` one-directionally (`xtask/src/check_ship_gate_completeness.rs:212-230`) and **never** iterates `needs`, so omitting the job from `EXPECTED_GATES` is silently allowed and nothing asserts every workflow job is enrolled somewhere |

### 18a. The fork round 3 opened — RULED

**R22's receipt ordering.** Mechanism (x) writes the `PlannedUnload` receipt before the hook signals the child.
Either (A) accept it and state in AC2 that the receipt attests *the kernel's release of the Spirit*, not the
child's death — which is what `PlannedUnload` already means for every in-process class, since none of them has a
child at all; or (B) split the cleanup — `revoke`/`drain`/`map.remove` ahead of the hook, `terminate_spirit`
**after** it, guarded so it runs on both the success and failure paths. (B) is more faithful and is a few lines
more; (A) is a one-sentence AC clarification. **Recommendation: (A)**, because `terminate_spirit`'s own
`TerminationKind::PlannedUnload` is already emitted for Spirits with no OS process, so the receipt has never
attested a process death — and (B) reintroduces a fallible step between the state flip and the receipt, which
is the exact shape R7 exists to remove.

✅ **RATIFIED by the operator 2026-09-19: (A).** The `PlannedUnload` receipt attests **the kernel's release of
the Spirit, not the death of an OS child** — written before `on_unload` signals anything, and correct by
construction, because `TerminationKind::PlannedUnload` is already emitted for every in-process class and none of
them has a child at all. AC2(g)(iii) states it in one sentence rather than implying a process-death guarantee
the receipt never carried. (B) is rejected on the §17 R7 ground: it puts a fallible step back between the state
flip and the receipt, which is the defect this story exists to close. **All four round-3 blockers are now folded
(AC1 R23, AC2(g) R21/R22, AC6 R24, §11 rows 8–9, T9); §18 leaves nothing open and the Status line is
unconditional.**

### 18b. Axes round 3 cleared

- **The kernel ceiling raise is mechanically possible.** `authorize_raise` (`xtask/tests/recovery_lane_ceiling_rule.rs:66-103`) is **fixture-only** — all seven callers are synthetic (`:128-204`); it never reads the real `kloc.toml`. The only tree-bound assertion is `kernel_core_ceiling_has_not_moved_under_the_easing` (`:266-281`, `assert!(actual <= RATIFIED_AT_EASING)` at `:275`), whose documented escape — move `RATIFIED_AT_EASING` in the same commit under the operator grant — was exercised by 16-3/16-4 and 16-5. Repo-wide, `19053`/`19_053` occur in exactly **two** places: `kloc.toml:247` and `recovery_lane_ceiling_rule.rs:268`.
- **But the raise IS required: headroom is ZERO, not `+26`.** `kloc.toml:247`'s own comment ends *"ZERO HEADROOM retained"*. `kloc-check` (`xtask/src/kloc_check.rs:220-388`) counts **tokei `code` lines** (comments and blanks excluded) with `-e target -e tests -e benches -e examples -e fuzz -e spirits`, so `crates/*/tests/` is free and **inline `#[cfg(test)]` is charged** — the `loaded_to_unloaded_rejected` rewrite consumes budget, as §15 V1 said.
- **The pin set reconciles: 97 `.rs` files under `crates/maos-kernel-core/src` + `security/sandbox/t3-image.lock` = the pinned 98** (`kernel-core-baseline.toml:543-640`, `PINNED_ENTRIES` `kernel_pin_content_hash_16_0.rs:33`), and `src_lines = 24628` is the **exact raw line sum** of those 97 files — verified by direct count at HEAD. So the pin is a plain sum and `--emit-pin` will round-trip.
- **No floor value bricks posture shifting.** `Posture` is `Cautious < Assistive < AutonomousWithHalt < Autonomous` (`maos-manifest/src/manifest.rs:645-652`), so `min()` is monotone-down; a floor of `Autonomous` is a no-op, and the tightest (`Cautious`) merely pins every Spirit to `Cautious` and makes `--shift assistive` return `above_posture_ceiling` — restrictive, **loud**, recoverable. ⚠ **RISK: the field named `floor` is implemented as a `min()`, i.e. a CEILING.** Anyone later "fixing" it to `max()` gets an instant brick at `Autonomous` via `shift_posture`'s `NonRuntimePosture` refusal (`cap_policy/mod.rs:239-241`). **Name it `operator_posture_ceiling`, or bound its legal values to `{cautious, assistive, autonomous-with-halt}`** — round 2 did neither.
- ⚠ **RISK: clamping `allowed_max` changes `posture_hash()`** (`security/posture.rs:108-110`, which hashes **both** fields), so a capability token minted against a pre-clamp snapshot verifies as `PostureMismatch`. Benign today **only** because admission precedes token issuance on both `maos run` arms — state it, do not assume it.

### 18c. Obligations this round adds

**(aj)** Demote the `?` at `scheduler_loop.rs:502` **without** the `on_unload` spawn guard, force a hook panic,
and confirm **two** receipt pairs with mismatched `kind_str` (`planned_unload` + `unplanned_crash`) for one pid —
then add the guard and confirm exactly one. **(ak)** Under mechanism (x), force a panicking `on_unload` and
confirm `handle_crash` returns `NotLoaded` (`crash_detector.rs:78-84`) and that `task.orphaned` +
`enforce_disposition` are therefore **absent** — then confirm the inline replacement emits them. **(al)** Call
`terminate_spirit` twice for the same `(pid, boot_nonce)` and confirm **two** distinct `HaltId`s and **four** TL
frames — the non-idempotence R22 rests on. **(am)** Run `one_daemon_one_door_16_1` with the posture floor set to
`cautious` and confirm `:390-403`'s `assert_ne!` **reds**; then confirm it passes with the floor null.
**(an)** Run the new CI job's steps on a runner with **no `target/` cache** and confirm `maosctl` is absent until
`cargo build --locked -p maos-cli --bin maosctl` is added. **(ao)** Set the posture floor and confirm the
operator can read **both** the requested and the effective ceiling off a real surface; delete the new key ⇒
confirm §16a's requirement becomes unverifiable, which is why it is a field and not a sentence.

---

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` (frontier-class allowlist).

### Debug Log References

**T0 environment repairs — the tree was NOT green as the story claimed, for two
reasons that are both tooling, not code.** (1) `kloc-check` shells out to
`tokei` and pins version **14.0.0**; it was not installed
(`failed to read tokei version`), and installing 15.0.0 first produced
`tokei version mismatch: expected 14.0.0, found 15.0.0`. (2) This workstation
sets `CARGO_TARGET_DIR=/mnt/build/cargo`, but `check_exit_commands.rs` resolves
binaries at the hardcoded `root.join("target/debug")`, so with the binaries
genuinely built the gate still reported `surfaces {maos:0, maosctl:0}`, four
`owed` `maos`/`maosctl` rows and 12 `unresolved-verb` findings. A `target →
$CARGO_TARGET_DIR` symlink (already gitignored) resolves it. Both are worth
naming for the next story on this machine.

**T0 measured at `471608ea`, every number re-derived:** run block
`main.rs:4094-6047`; `classify_spirit` `:456` with 8 classes;
`SCANNED_SOURCE_FILES` 23; pin `src_lines` **24628** on line **505**, 98 files,
`set_hash 66ee5370…`; `RATIFIED_AT_EASING` **19_053** (at `:268`, not the
story's `:259`); aggregate **167189**, hardfail 170884; `maos-bin` 22996/22996
= **ZERO**; `maos-kernel-core` 19053/19053 = **ZERO**; `maos-cli` 6080/6856;
`maos-control` 1264/1791; `maosctl` surfaces **25**; `check-exit-commands` 31
resolved with `load` **owed to `16-6 [ready-for-dev]`**. `--emit-pin` confirmed
to round-trip **byte-identical** over lines 538-640 before a kernel line was
written.

**Proven-red captured BEFORE the kernel arm (obligation (b)).**
`crates/maos-kernel-core/tests/loaded_spirit_exit_16_6.rs` red at HEAD with
`InvalidStateTransition { spirit_id: "door-loaded", current: Loaded, verb: Unload }`,
and the hook-failure falsifier red because `on_unload` never dispatched at all
(the `?` returns before `fire_on_unload`). `terminate_spirit`'s
non-idempotence (obligation (al)) PASSED at HEAD — it is characterisation, and
it is why AC2(g)(i) guards the spawn rather than making the function
idempotent.

**Falsifications run, each restored afterwards.** (c)/(w) deleting the
`(Loaded, Unloaded)` arm reds `loaded_to_unloaded_allowed` AND both AC2
integration falsifiers while the AC1 posture suite stays 5/5 green — the two
ACs are separable and the grant buys exactly one thing. (ac) clamping
`allowed_max` only reds exactly one test,
`a_default_above_the_ceiling_is_clamped_too_never_left_inverted`. (l)/(ah)
deleting the `route_budget` `Load` arm compiles with **zero** errors, con­firming
R11: that match ends `_ => DEFAULT_ROUTE_BUDGET` and the arm is
uncompiled-checked. (ag) any mention of the policy-scope table in
`admission.rs` reds `composition_root_does_not_seed_manifest_scopes` — which
also caught the module's own doc comment, since the negative greps raw source
including comments. (u) prepending a two-line HISTORY block above `src_lines`
reds the position assert at line 507.

⚠ **(j) is an HONEST NEGATIVE and is recorded as one.** Deleting the door's
pre-CAS lock key and re-running the concurrent-load test still PASSES: two
`maosctl` process spawns do not interleave tightly enough to land inside the
kernel's `resolve_pid`/`insert` window. The test asserts the contract but does
not discriminate the lock; the lock's necessity rests on the measured shape of
`SpiritSchedulerAdapter::load`, not on that test going red. Stated in the
test's own doc comment rather than left as an implied proof.

**Two null controls found and repaired, both pre-existing.**
(1) `production_collective_calls_share_one_atomic_pid_binding`
(`cohort_daemon_smoke_13_5c.rs`) `include_str!`d `main.rs` and searched for
`fn collective_write(` — measured **0 occurrences in main.rs at HEAD**, 3 in
`cross_team_crossing.rs` where the methods were relocated. It had been
panicking on its first iteration and asserting nothing since that move.
Re-pointed at both owning files and **falsified**: planting a second
`self.spirit_pid.load(` reds it with the right message. (2)
`production_capability_parsers_are_all_schema_degraded` counted parsers in
`main.rs` alone; with the extraction it now spans `main.rs` + `admission.rs`
(4 parsers, 4 degrade sites — the count drops 5 → 4 because the two
`caps_required_or_empty` call sites collapsed into one shared path).

**Regression triage.** Five suites fail on this machine; **all five fail
identically at HEAD**, verified in a detached `git worktree` at `471608ea`
with matching pass/fail counts: `two_host_reconcile_2c` (8/2, python twin),
`equiv_harness` (17/3) and `t2_sandbox_kill` (2/1) (wasm host),
`abi_diff_integration` (0/5), and `service_boundary_integration` (the
pre-existing `KernelHaltResolver` removed+unclassified pair). ⚠ Worktrees
share `CARGO_TARGET_DIR`, which contaminated one intermediate run with HEAD's
`maos-control` rlib; `cargo clean -p` on the five touched crates and a full
re-run produced the final result.

**One surface baseline moved, one line.** Adding `operator_posture_ceiling`
changes `OperatorPolicyConfig`'s signature hash, which
`check-service-boundary` reads as a removed symbol.
`docs/ci-baselines/kernel-surface-v0.1-beta.json` re-pinned
`7ba7c38c…` → `21bbe125…`; the change is additive (the field is
`Option<Posture>` defaulting `None`) and the story's own violation is gone.

### Completion Notes List

- **AC1 — one extracted admission path, and it really is one.** New
  `crates/maos-bin/src/admission.rs` owns gates 1-8 (`read_manifest`,
  `gate_manifest`, the model-provenance leg) and the whole
  `load → admit → start` triple with rollback. **Three** callers share it:
  `maos run`'s standalone arm, `maos run`'s topology arm (§14 Q5 = Shape A)
  and the operator door's `load`. Shell is out of scope by name — it parses a
  compiled-in manifest string and hard-codes `HelloSpirit`.
  - The triple takes a `StartPolicy`: `maos run` passes `StartNow`, the door
    passes `LeaveLoaded`. That one bit is the only difference between the
    callers, and it is what keeps `load` and `start` two of FR9's five verbs.
  - Admission runs at the SCHEDULER-ASSIGNED pid on every path, because
    capability mediation is keyed by pid. Both `maos run` arms adopt the
    standalone ORDER (provenance before the load), so topology's refusal no
    longer happens after a pid exists, and its `emit_vetter_key_event` now
    fires on **rejection** as well as grant — at HEAD a refused topology
    admission wrote no vetter row at all.
  - Rollback: any failure after the load unwinds through `scheduler.unload`,
    which is only expressible because of AC2's kernel arm. The
    partial-admission leak at `main.rs:5788` and the missing transition were
    the same defect from two ends, and both close here.
  - ⚠ **A kernel entry point was required and is inside the granted file.**
    `load` is generic over `T: Spirit`, so a caller holding an
    `Arc<dyn AnySpiritObj>` (the shape class dispatch produces) could not
    reach it without a second per-class copy of the triple. `load` is now a
    monomorphising wrapper over a new type-erased `load_obj`; behaviour for
    every existing caller is unchanged.
  - §15 V4 / §17 R20 closed as a **type guarantee**, not a dev note:
    `build_spirit_obj` dispatches on `LoadedSpiritKind` exhaustively, so a
    ninth class cannot be added without a loader. The missing `researcher`
    arm is added. `story_16_6_known_class_names_match_classify_spirit` pins
    the one edge the compiler cannot see.
- **AC2 — the kernel arm and the receipt gap.** `(Loaded, Unloaded)` added;
  `loaded_to_unloaded_rejected` rewritten as `loaded_to_unloaded_allowed` in
  the shape of its five siblings. AC2(g) ships mechanism **(x)**: receipt,
  revocation, halt drain and map removal all run AHEAD of the fallible
  `on_unload`, `check_hook_outcome` never spawns `handle_crash` for
  `on_unload` (two teardowns of one pid mint two `HaltId`s), and FR50
  disposition + `task.orphaned` are performed inline by a new
  `orphan_in_flight_tasks`. §18a (A) is stated in the code: the
  `PlannedUnload` receipt attests the kernel's release of the Spirit, not the
  death of an OS child.
- **AC3 — `start` has a subject.** `load_start_pause_resume_unload_against_a_live_root`
  runs FR9's five verbs against one Spirit on a real `maos run` daemon, and
  `start_on_a_running_spirit_is_refused_with_the_d16_1_j_conflict` keeps the
  409 half. The AC3 claim is worded **operator-creatable**, never *existent*.
- **AC4 — the door surface.** `OperatorCommand::Load { manifest }` with arms in
  all three exhaustive matches (`spirit_key` ⇒ `None`, `dispatch`,
  `command_verb` ⇒ `"load"`) plus the compiler-unenforced `route_budget` arm.
  ⚠ **D-16-6-D is implemented one layer up from where the story put it**: the
  lock key is peeked from the manifest's `[class].name` in `run_command`
  **before** the `QUEUED → STARTED` CAS, so a contended `Load` loses the port
  lock and is withdrawn as `spirit_busy` like every other verb. Taking it
  inside the handler would have put it after the CAS — unbounded, and
  answering `handler_still_running`, which claims the command started.
  `BinPrivateOps` widened with `security`, `pid_by_spirit_id` and
  `load_spirit` (§17 R13). R23(i)'s sixth key (`posture_ceiling`) and R2's
  `lifecycle_states` sibling key both ship; the three compile-caught struct
  literals R18 predicted were exactly the three that broke.
- **AC5 — the pin moved honestly.** `src_lines` 24628 → **24796**, still on
  line **505**; the 16-6 HISTORY row sits BELOW the assignment precisely
  because every prior re-pin prepended one and one more would have pushed it
  off. FOUR digests changed by name, `PINNED_ENTRIES` stays 98, four `24628`
  literals updated, `RATIFIED_AT_EASING` 19_053 → **19_121**.
- **AC6 — the gate binds.** `check-exit-commands` flipped `load` from
  **owed** to **resolved** (31 → 34 tokens, `maosctl` surfaces 25 → 26, zero
  findings), and `owed` is back to the 18-2/20-1 entries. Named CI job
  `maosctl-load-over-the-door` enrolled in `v1-0-ship-gate.needs` only, **with
  the `cargo build --locked -p maos-cli --bin maosctl` step R24 proved the
  16-1 template is missing**.
- **Measured budget.** `maos-bin` +275 against a story FLOOR of +420 — the
  extraction genuinely collapsed two call sites. `maos-kernel-core` +68,
  `maos-cli` +25, `maos-control` +22, **`maos-domain` ZERO** (obligation (v):
  no typed error leaked). Aggregate 167189 → **167595**, leaving +3289 to
  hardfail; the alarm is still firing and was **not** silenced.
- **Not applied, and why:** the `rs-parking-lot` house rule. `spirits` is
  exposed as `pub fn scbs() -> Arc<std::sync::RwLock<…>>` with ~30 external
  call sites, so converting it is a public-API change outside this story's
  enumerated kernel grant. The new code matches the file's existing
  convention.

### File List

**New**
- `crates/maos-bin/src/admission.rs`
- `crates/maos-bin/tests/maosctl_load_16_6.rs`
- `crates/maos-cli/tests/load_test.rs`
- `crates/maos-kernel-core/tests/loaded_spirit_exit_16_6.rs`
- `crates/maos-kernel-core/tests/operator_posture_ceiling_16_6.rs`

**Modified — kernel-core (◆ FLAG-Winston, four files)**
- `crates/maos-kernel-core/src/scheduler/control_block.rs`
- `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs`
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`
- `crates/maos-kernel-core/src/security/mod.rs`

**Modified — production**
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/operator_door.rs`
- `crates/maos-bin/src/shell_host.rs`
- `crates/maos-bin/src/supervision.rs`
- `crates/maos-cli/src/cli.rs`
- `crates/maos-cli/src/subcommands.rs`
- `crates/maos-control/src/lib.rs`
- `crates/maos-audit/src/fr4_classifier.rs`

**Modified — tests**
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/shell_host_16_2.rs`
- `crates/maos-control/tests/post_surface_16_1.rs`
- `crates/maos-control/tests/submit_and_wait_16_2.rs`
- `crates/maos-kernel-core/tests/manifest_field_coverage.rs`
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/tests/recovery_lane_ceiling_rule.rs`

**Modified — pins, budgets, CI, planning**
- `xtask/kernel-core-baseline.toml`
- `xtask/kloc.toml`
- `docs/ci-baselines/kernel-surface-v0.1-beta.json`
- `.github/workflows/discipline.yml`
- `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md`
- `_bmad-output/planning-artifacts/epics/requirements-inventory.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/16-6-maosctl-load-over-the-door.md`

### Change Log

| Date | Change |
|---|---|
| 2026-09-19 | T0 re-measured at `471608ea`; two environment repairs (`tokei` 14.0.0, `target` symlink for `CARGO_TARGET_DIR`); AC2 falsifiers captured RED before any kernel edit. |
| 2026-09-19 | T3 kernel (◆ FLAG-Winston, four files): `(Loaded, Unloaded)`; the inverted unit test rewritten; `unload` cleanup moved ahead of `on_unload` with the crash-spawn guard and inline FR50 disposition; type-erased `load_obj`; `operator_posture_ceiling` + its two admission `min()`s. |
| 2026-09-19 | T2: new `crates/maos-bin/src/admission.rs`; both `maos run` file-manifest arms re-pointed at it; class dispatch unified on `LoadedSpiritKind` (exhaustive), closing the `researcher` gap as a type guarantee. |
| 2026-09-19 | T4/T5: `OperatorCommand::Load`, `POST /v1/spirits`, `BinPrivateOps::load_spirit`, the pre-CAS lock key, the `posture_ceiling` and `lifecycle_states` keys, and the `maosctl load` verb. |
| 2026-09-19 | T6: `maosctl_load_16_6.rs` (6 tests on a live root), `load_test.rs` (4 CLI contract tests), named CI job with the `maosctl` build step R24 proved missing, `SCANNED_SOURCE_FILES` 23 → 24. |
| 2026-09-19 | T3 re-pin: `src_lines` 24628 → 24796 on line 505, four digests, four literals, `RATIFIED_AT_EASING` → 19_121, measured ceilings `maos-kernel-core` 19121 / `maos-bin` 23271. |
| 2026-09-20 | Code review (6-layer net: Blind + Edge Case + Acceptance + Test-Infra + Security + runtime re-execution; 49 raw → 28 findings, 5 team rulings at the code-review-crew round-table): 27 patches applied. Kernel: posture-ceiling WRITER (PostureSection parser + PolicyTable setter + re-clamp; the clamp is no longer dead code), `LifecycleError::HookPanicked` teardown-ownership (rollback never double-unloads a panicking `on_start`), replica resolver plumbed to the inline FR50 disposition, honest `PlannedUnload` receipt comment. Door: typed `AlreadyLoaded`/`ClassBuild`/`EpistemicPortRequired` refusals, epistemic-port parity gate (Mira/Researcher refuse where `maos run` fails loud), single-read manifest handoff (`spawn_blocking`, size cap, validated lock key — kills the peek DoS + TOCTOU), audit row carries the loaded Spirit's id/pid, journal failures answer success-with-warning, rollback on post-insert `load_obj` failure (the §2 leak closed on its last leg). Tests: real CrashDetector wired (the on_unload guard now reds when deleted), `orphan_in_flight_tasks` covered with the exact FR4 payload shape, admission-rollback vector (`ESubstrateTooOld`), roster-absence + distinct-code assertions, POST /v1/spirits route guards (405/411/413/400/404), `posture_ceiling` rendered by `maosctl spirit inspect` + key pinned, per-file parser/degradation pairing, canonicalisation test now discriminates. Docs: cookbook + spirit-abi `on_unload` post-release semantics; epic §12 rows 7/8 (Kernel-Δ, Kloc asks) actually applied; devcontainer/`.gitignore`/memlog hunk split OUT of this commit (parked `../dev_ws-hold-16-6-devcontainer/`, to land hardened: pinned+checksummed installers, config/known_hosts-only ssh mount). Gates re-booked: kloc ceilings maos-bin 23271→23371, maos-domain 9268→9270, maos-kernel-core 19121→19213; kernel-core baseline 24796→24918 (4 digests, src_lines held on 505, HISTORY row below); pin literals 24796→24918. `lock_wait = deadline − 250 ms` bounded-wait restored (review agent's `try_lock` deviation caught by `shell_host_16_2`). Suites green: kernel-core 589, domain 294, control 40, audit 237, `maosctl_load_16_6` 8, `shell_host_16_2` 10, pin 24+10; `kloc-check` + `check-kernel-baseline` + `check-env-contract` PASS; maos-bin's 1 package failure remains the pre-existing `worker-cli-fixture` env gap. |

### Review Findings

**Review run 2026-09-20 — 6 layers (Blind + Edge Case + Acceptance + Test-Infra + Security + non-author runtime re-execution) on the uncommitted diff vs `471608ea`. 49 raw findings → 28 deduplicated; 1 dismissed as noise. Runtime: kernel-core 586/586, control 39/39, `maosctl_load_16_6` 6/6, `kloc-check` + `check-kernel-baseline` PASS.**

**Decision-needed**

- [x] [Review][Patch] Operator posture clamp is dead code — nothing ever writes `operator_posture_ceiling` — The two `min()`s are mechanically sound but the only writer in the tree is the new test (`operator_posture_ceiling_16_6.rs:47`); the field doc cites an operator-config parser that does not exist; the GET `posture_ceiling` surface can only ever echo the unclamped value; no re-clamp after admission. The manifest-chooses-its-own-posture escalation §16a targets stays open on every real boot. blind+edge+security (SEC-16-6-01) — **RESOLVED 2026-09-20 (team consensus): WIRE A REAL WRITER** — extend `security/operator_config.rs` to parse the ceiling, seed `PolicyTableInner` at composition root, re-clamp on change.
- [x] [Review][Patch] Mechanism (x) ordering vs the published hook contract and subprocess Workers — receipt/revocation/`spirits.remove` now run before `on_unload` (§18a ruled (A): the receipt attests kernel release, not child death). Consequences: (a) `KernelCtx::heartbeat` returns `ScbNotFound` for the whole hook and capability-mediated cleanup fails against a revoked pid, contradicting `docs/cookbook/lifecycle-hooks.md:67` and `maos-spirit-abi/src/lifecycle.rs:205`; (b) `WorkerSpirit.on_unload` is the ONLY thing that ends the OS child (`supervision.rs:1668-1680`) — a panicking/budget-exceeded Worker hook leaves a live child with no SCB and a receipt already written; (c) the justifying comment's "none of which has a child at all" is false — Worker flows through this path. blind+edge — **RESOLVED 2026-09-20 (team consensus): ACCEPT + DOCUMENT** — keep the ratified order; fix the false comment; update cookbook + spirit-abi docs to state `on_unload` runs post-release without ctx capabilities; record the Worker window in deferred-work.
- [x] [Review][Patch] Rollback after a panicking `on_start` races the crash teardown that hook spawned — `check_hook_outcome`'s no-spawn guard covers `on_unload` only; for `on_start` panic it spawns `handle_crash` AND returns `Err`, then `load_admit_start` rollback calls `scheduler.unload` concurrently. `terminate_spirit` is not idempotent (`HaltId` embeds wall-clock): double receipt/mismatched halt frames, or the crash record lost (`NotLoaded` into a discarded `Result`). Needs a teardown-ownership rule. edge — **RESOLVED 2026-09-20 (team consensus): CRASH-HANDLER OWNS TEARDOWN** — `Panicked` ⇒ no rollback-unload (handler owns it); `BudgetExceeded`/others ⇒ rollback unloads; document the classification-loss residual of the fire-and-forget spawn.
- [x] [Review][Patch] Journal/approval-log failure after a successful load reports `Failed` while the Spirit stays admitted — `load_command` returns `journal_failed`/`approval_log_failed` with no unwind; contradicts `admission.rs`'s "a refusal leaves NOTHING behind" framing from the operator's seat. Unwind vs success-with-warning needs a ruling. blind — **RESOLVED 2026-09-20 (team consensus): SUCCESS + WARNING** — `journal_error`/`approval_log_error` in the Completed payload + stderr note; a Failed exit that leaves a running Spirit is the false record (16-5 doctrine).
- [x] [Review][Patch] ~1,240 unclaimed lines ship in this commit — 12 `.devcontainer/*` files, the `.gitignore` entry and `.memlog.md` (new lines carry UTF-8 mojibake) appear in no File List section; the devcontainer image installs two third-party CLIs as root via unpinned `curl | sh` (`Dockerfile:63-69`) and bind-mounts the host `~/.ssh` directory under a comment claiming private keys never enter the container (`docker-compose.yml:31-42`). auditor+blind — **RESOLVED 2026-09-20 (team consensus): SPLIT OUT OF THIS COMMIT** — revert the `.devcontainer/*` additions, the `.gitignore` entry and the memlog hunk here; land separately with pinned+checksummed installers and a config/known_hosts-only mount.

**Patch**

- [x] [Review][Patch] Refuse door loads whose `[epistemic_policy]` requires the scalar halt port — Mira/Researcher arms build portless objects; `maos run` treats this exact condition as FATAL boot; `spirits/mira/manifest.toml:79-87` declares such a halt, so `maosctl load` + `start` admits a Spirit whose halt can never fire (AC1 gates 9/10 violation) [crates/maos-bin/src/main.rs:825]
- [x] [Review][Patch] Roll back the SCB when `load_obj` fails after inserting it — `on_load` failure strands a `Loaded` SCB holding the id while the door answers 400 under a "left nothing behind" comment [crates/maos-bin/src/admission.rs:598]
- [x] [Review][Patch] Remove the `pid_by_spirit_id` entry on door unload — the map is insert-only outside the rollback path; the I12 digest provider closure (`main.rs:2470-2485`) keeps attributing decisions to a released pid [crates/maos-bin/src/admission.rs:656]
- [x] [Review][Patch] Put the loaded Spirit's id/pid on the load completion audit row — `command.spirit_key()` is `None` for `Load`, so the one durable row per mutating command carries `spirit_id: ""`, `spirit_pid: 0`; `lock_key` is in scope [crates/maos-bin/src/operator_door.rs:1488]
- [x] [Review][Patch] Guard `peek_spirit_id` and read the manifest once — no regular-file check, no size cap, blocking read on the async runtime before any gate (FIFO hang / `/dev/zero` slurp = door-wide stall), and lock key derived from a different read than the load (TOCTOU) [crates/maos-bin/src/admission.rs:356]
- [x] [Review][Patch] Type the `AlreadyLoaded` refusal and stop echoing raw file source lines — 409-vs-400 rides on a Display substring of another crate's error; TOML parse errors quote the target file's contents into door bodies [crates/maos-bin/src/admission.rs:259]
- [x] [Review][Patch] Give class-construction failures their own refusal code — `build_spirit_obj` errors reuse `load_refused`/"scheduler refused the load" though the scheduler was never called [crates/maos-bin/src/main.rs:8589]
- [x] [Review][Patch] Pass the replica resolver to the inline FR50 `enforce_disposition` — `None` makes `ReassignToReplica` always escalate while the `task.orphaned` row still reports "reassign" [crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:709]
- [x] [Review][Patch] Wire a real CrashDetector into the `loaded_spirit_exit_16_6` harness — `crash_detector: None` makes the new on_unload crash-spawn guard dead code in its own test; deleting the guard leaves the file green (obligation (aj) falsifier vacuous) [crates/maos-kernel-core/tests/loaded_spirit_exit_16_6.rs:97]
- [x] [Review][Patch] Cover `orphan_in_flight_tasks` with a non-empty in-flight ledger — the function early-returns empty in every test; AC2(g)(ii)'s ~45 lines, payload key set and disposition call never execute [crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:672]
- [x] [Review][Patch] Add a refusal vector that fails AFTER the load to exercise `rollback` — all four vectors fail in `gate_manifest` pre-pid; `rollback`, `AdmissionRefusal::Admission`/`::Start` are executed by no test (e.g. out-of-window `manifest_schema_version`) [crates/maos-bin/tests/maosctl_load_16_6.rs:505]
- [x] [Review][Patch] Assert the refused Spirit is absent — the suite checks only `butler == Running`; the refused id is never queried and `baseline` is never compared (a stranded `Loaded` SCB passes green) [crates/maos-bin/tests/maosctl_load_16_6.rs:499]
- [x] [Review][Patch] Assert each vector's distinct typed refusal code, not `contains("HTTP 400") || contains("HTTP 409")` [crates/maos-bin/tests/maosctl_load_16_6.rs:495]
- [x] [Review][Patch] Assert the contended-load loser answers `spirit_busy` within `lock_wait` — `winners == 1` is satisfied by `handler_still_running` or a bare 409 too (AC4/§17 R12/(ad)) [crates/maos-bin/tests/maosctl_load_16_6.rs:427]
- [x] [Review][Patch] Test POST /v1/spirits method/length/body refusals in the owning crate — 405/411/413/400/404 arms untested anywhere; post_surface_16_1.rs only gained struct fields [crates/maos-control/src/lib.rs:1056]
- [x] [Review][Patch] Render `posture_ceiling` in `maosctl spirit inspect` and pin the GET key — inspect still prints five fields; no test asserts the sixth key, so deleting it breaks nothing (R23(i)/(ao)) [crates/maos-cli/src/subcommands.rs:2429]
- [x] [Review][Patch] Apply epic §12 rows 7/8 — the Kernel-Δ and Kloc-asks paragraphs still read "ZERO kernel-Δ for 16-1..16-4" and the stale 166542/+4342 aggregate while the story marks both APPLIED and task T1 ticked [_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md:61]
- [x] [Review][Patch] Make the canonicalisation test discriminate — the argument is already absolute and canonical, so the assertion passes even if `canonical_manifest` were removed [crates/maos-cli/tests/load_test.rs:56]
- [x] [Review][Patch] Enforce capability-parser/degradation pairing per file — the cross-file sum passes when one file drops a degradation and another gains one (N-1 manifest admitted undegraded) [crates/maos-kernel-core/tests/manifest_field_coverage.rs:258]
- [x] [Review][Patch] Assert the concrete effective tier instead of `assert_ne!(T4)` — the negative passes for every wrong answer except T4 [crates/maos-kernel-core/tests/operator_posture_ceiling_16_6.rs:247]
- [x] [Review][Patch] Test hygiene in loaded_spirit_exit_16_6.rs — assert or drop the unread `unload_calls` counter (:153); drop or harden the wall-clock halt-id characterisation test (:274, `SystemTime` non-monotonic) [crates/maos-kernel-core/tests/loaded_spirit_exit_16_6.rs:153]
- [x] [Review][Patch] Capture the FR4 `task.orphaned` fixture from a real emission — the hand-written payload classifies identically with the new table row deleted; drift between writer and table is undetectable [crates/maos-audit/tests/fr4_classifier_16_2.rs:211]

**Deferred**

- [x] [Review][Defer] Two pre-existing test-environment gaps, not caused by this change [crates/maos-cli/tests/two_host_reconcile_2c.rs:511; crates/maos-bin/tests/drain_once_audit_writer.rs:27] — deferred, pre-existing: `maos-cli` 2 failures need a Python Ed25519 backend (`pip install cryptography`/`PyNaCl`); `maos-bin` package run stops at `drain_once_audit_writer` on missing `worker-cli-fixture`. Story target `maosctl_load_16_6` passes 6/6 when run separately.
