---
baseline_commit: 5ccd862c
depends_on: 13-5b-collective-tier-erasure-legal-hold-cascade
kernel_grant: AUTHORIZED 2026-07-25 — FLAG-Winston, pin 23228 → 23234 (+6), Variant B
---

# Story 13.5h — Shared-tier principal partition: kill the discharge that is green in both worlds

Status: **done** — review completed 2026-07-26; all 4 review findings patched, targeted regressions and the Reza production gate green.
**Kernel-Δ: AUTHORIZED bounded FLAG-Winston re-pin `23228 → 23234` (+6 measured).** Operator-ratified 2026-07-25 on a **unanimous 4/4** panel (Winston 🏗️ · Murat 🧪 · John 📋 · Amelia 💻). The KLOC ceiling was **not** re-based *for this grant* — Variant B fit under the then-standing 17 900. It has since moved to **18 248** by the separate founder-ratified ceiling-policy replacement of 2026-07-25 (see *Budget*), which is a capacity change, not authorization: this story's authorized delta is unchanged at +6. Filed from the 13.5b preflight (D13 / Residual 2) and its code review (ADR-059 Decision 8 / Residual 2).

> **Both open questions are closed. This story is ready to implement as specified.**
>
> **The fork was settled by evidence.** The 13.5b preflight filed this story with an open fork — *(i) partition* vs *(ii) erase* — and one condition: *"Fork (i) is expected to win, but only if no legitimate use case requires principal data in the Shared tier. Answer that question at the successor's preflight; do not assume it."* A whole-workspace search for a call site passing `MemoryTier::Shared` together with `MemoryNamespace::Principal` returns **nothing** — not in `crates/`, `examples/`, `xtask/`, fixtures, or tests. Every executable Shared write in the repository is `MemoryNamespace::Coordination`: `memory/mod.rs:999-1008`, `crates/maos-kernel-core/tests/memory_three_tier_smoke.rs:48-58`, `crates/maos-audit/tests/multi_backend_erasure_test.rs:106-118`. `xtask/tests/story_10_4a_ac1_proven_red.rs:562,582` reads and scans Shared but never writes it. The architecture inventory places the principal namespace **in the private tier** (`requirements-inventory.md:337-338`), and so does ADR-026 itself (`docs/adr/ADR-026-principal-memory-namespace.md:12`). **Fork (i) PARTITION wins; fork (ii) ERASE is rejected** — it would have to distinguish principal rows inside a `namespace TEXT` column with no `principal_id` (`shared.rs:16-27`), in a tier whose whole point is cross-Spirit sharing, to delete data that does not exist.
>
> **The kernel cost was measured, not estimated, and the grant is issued.** Both candidate shapes were built against the real tree, `cargo fmt`-ed, compiled, tested, measured, and reverted. See *Ratified implementation shape* — the panel chose the shape that is **4× cheaper and architecturally stronger**, and the story's original assumption (Variant A) was rejected 4/4.

---

## Story

**As** the operator answering a GDPR Article 17 request on a MAOS Host,
**I want** the Shared tier to refuse subject-scoped PII at the same boundary the Collective tier already refuses it, and the erasure evidence base to *prove* that refusal rather than assert it,
**so that** the signed proof-of-erasure and the `covered ⊎ retained == registered` partition stop being satisfiable in a world where unerasable principal data is sitting in `shared_memory`.

---

## The defect, in code (verified 2026-07-25 against `5ccd862c`)

### D-1 — Decision D was applied to one of the two tiers it names

`reject_principal_collective` (`crates/maos-kernel-core/src/memory/mod.rs:171-185`) is called at **six** collective entry points: `:242`, `:306`, `:351`, `:745`, `:805`, `:859`. Its own doc gives the reason — extending the forget cascade into a tier that cannot erase *"would open a GDPR Art.15/17 hole."*

The Shared arms, one line above, are bare delegates with **no guard at all**:

```rust
mod.rs:743   MemoryTier::Shared => self.shared.write(spirit_pid, namespace, key, value),
mod.rs:803   MemoryTier::Shared => self.shared.read(spirit_pid, namespace, key),
mod.rs:857   MemoryTier::Shared => self.shared.scan(spirit_pid, namespace, prefix, limit),
```

`SpiritMemoryView` exposes exactly three methods — `write`/`read`/`scan` (`for_spirit.rs:31-59`) — and each takes `tier` as a **caller-supplied** parameter. The PID is kernel-fused (`mod.rs:397-398`); the tier is not. So a Spirit may write `(MemoryTier::Shared, MemoryNamespace::Principal { .. })` and it is accepted. `subject_access`, `forget`, and `export_redactable` take no tier (`crates/maos-domain/src/ports/memory.rs:81-101`) and never reach Shared. **The partition surface is three sites, not one.**

### D-2 — the tier it lands in has no erase path at any visibility

`crates/maos-kernel-core/src/memory/shared.rs` complete API: `pub fn open` (`:41-49`), `pub(in crate::memory) fn write` (`:123-149`), `read` (`:153-188`), `scan` (`:191-250`), plus seven private helpers. **No delete, erase, forget, or remove at any visibility.** The forget cascade has nothing to call, which is why 13.5b had to record `shared` as `CategoryStatus::CoverageGap`.

### D-3 — NULL CONTROL #23, and it is **measured**, not argued

`multi_backend_erasure_test.rs` discharges `"shared"` like this:

- `:106-117` plants the canary under **`MemoryNamespace::Coordination`** — a *non-principal* namespace, self-described at `:106-108` as *"legitimate cross-Spirit data that ADR-026 says must NOT be erased."*
- `:143-146` asserts that row **survived** the forget.
- `:147` pushes `"shared"` into `proved_principal_empty`.

Surviving legitimate Coordination data proves **no over-erasure** (ADR-026). It says nothing about principal-emptiness.

**Proof, run 2026-07-25.** Swap `&MemoryNamespace::Coordination` → `&principal_ns` on `:113` — nothing else — and the suite reports `test result: ok. 1 passed; 0 failed`. The write lands (bare delegate; no rejection in `shared.rs:123-149`), the forget ignores it (`mod.rs:569-572` touches only Private + `principal_index`), and `shared_store_contains` scans every `shared_memory` value **with no namespace filter** (`:64-86`), so the assertion at `:143-146` still finds its canary. **`"shared"` is still pushed into `proved_principal_empty`, and the disjoint-union invariant at `:166-187` still passes — with unerasable principal PII in the tier.** Green in both worlds.

Re-run **with the ratified fix applied**: the same swap now makes the test **FAIL at `:117`**. The control discriminates.

⚠ 13.5b's review tightened the **Collective** discharge (`:149-164`) and left the Shared discharge **byte-for-byte untouched**.

### D-4 — it is an Article 15 hole as well as Article 17

`subject_access_query` (`crates/maos-audit/src/lib.rs:1344-1373`) selects **only** from `principal_index` (`:1357-1361`). Index rows are recorded **only in the Private arm** (`mod.rs:729-742`); the Shared delegate at `:743` records nothing. A Shared-tier principal row is invisible to subject access, so FR45's *"0 leakage in 100 follow-up subject-access queries"* floor is met **vacuously** while the data sits durably in `shared_memory`.

### Honest severity

**Reachable, not reached.** No production Spirit, example, fixture, or test writes `(Shared, Principal)` today. This is a latent hole plus a control that cannot detect it plus a signed artifact that is wrong regardless. **Do not overstate it as an active leak.**

---

## Ratified implementation shape — Variant B (panel 4/4)

Both shapes were built, compiled, `cargo fmt`-ed, tested, measured, and reverted on 2026-07-25.

| | **A** — separate `reject_principal_shared` mirror | **B** — hoisted `reject_principal_outside_private` ✅ |
|---|---:|---:|
| physical (pin) | 23228 → 23254 (**+26**) | 23228 → **23234 (+6)** |
| tokei code | 17890 → 17910 (**+20**) | 17890 → **17894 (+4)** |
| vs the then-17 900 ceiling | **BREACH by 10** | **fits, 6 spare** |
| authorizations needed | **two** (pin + ceiling) | **one** (pin) |

**Variant A was rejected 4/4 on architecture, not on cost.** Amelia: under A a future fourth `MemoryTier` gets a bare-delegate arm that *silently admits* `Principal`; under B the pre-dispatch default is **deny**, and admitting a new tier requires an explicit, reviewable allowlist change. Winston: *"That is a structural improvement, not merely a smaller diff."*

### The change

Generalize the existing helper — rename it, take the tier, state the rule as a **negation of the allowlist**:

```rust
    /// Reject `Principal` namespace outside the private tier (Decision D).
    /// ONLY the private tier has a forget cascade and a `principal_index`; the
    /// shared and collective tiers hold cross-Spirit data with no erase path,
    /// so admitting subject-scoped PII there opens a GDPR Art.15/17 hole.
    /// Stated as a negation of the allowlist so a FUTURE tier is
    /// principal-rejecting by default.  Partitioned by construction.
    fn reject_principal_outside_private(
        tier: MemoryTier,
        namespace: &MemoryNamespace,
    ) -> Result<(), MemoryError> {
        if !matches!(tier, MemoryTier::Private)
            && matches!(namespace, MemoryNamespace::Principal { .. })
        {
            return Err(MemoryError::NamespaceViolation(format!(
                "Principal namespace is partitioned out of the {tier:?} tier \
                 (GDPR Art.15/17 — non-private tiers hold cross-Spirit data, not \
                 subject-scoped PII, and have no erase path)"
            )));
        }
        Ok(())
    }
```

Then, mechanically:
- **Hoist one call** above each of the three `match tier` blocks (`mod.rs:727`, `:801`, `:855`): `Self::reject_principal_outside_private(tier, namespace)?;`
- **Delete** the three in-arm Collective calls (`:745`, `:805`, `:859`) — the hoisted guard subsumes them.
- **Update** the three cap-gated call sites (`:242`, `:306`, `:351`) to `Self::reject_principal_outside_private(MemoryTier::Collective, namespace)?;` — Winston's rider: they must call the generalized helper *"so they do not become an alternate bypass."*
- The Shared arms stay **one-line delegates** — they are only reached after validation.

**The `format!` costs nothing extra.** Amelia verified `MemoryError::NamespaceViolation`'s payload is already an owned `String` (`crates/maos-domain/src/memory.rs:278-279`), so the current `.into()` already allocates. The old message text is asserted **nowhere** (grep-verified); `multi_backend_erasure_test.rs:161` asserts only `.contains("partitioned out")`, which the new text preserves. Murat's rule: **tests assert the typed refusal, never a frozen diagnostic sentence.**

### Verification already performed on Variant B

- Probe: Shared refuses `Principal` on **write, read and scan**; `Coordination` writes still succeed (ADR-026 preserved); Private principal writes unaffected.
- `cargo test --workspace --no-fail-fast`: **3500 passed / 2 failed / 92 ignored across 450 suites.** The only two failures are the pin guards (`t12b_kernel_core_byte_identical_line_count`, `frozen_snapshot_and_current_kernel_baselines_are_independently_valid`) — i.e. *"the pin has not moved yet."* **Zero behavioural regressions.**
- After re-pinning the one literal: `check-kernel-baseline`, `kloc-check`, `check-reza-production-path`, `check-multi-tenant-loom`, the a2a-tcp guard and the fkcs oracle **all pass**.

---

## Acceptance Criteria (5)

**AC1 — The grant is spent exactly as authorized, and recorded.**
The FLAG-Winston escalation is **already done** — see *Authorization record*. Task 0 is now bookkeeping, not escalation. Binding conditions, all seven:
1. **Variant B only.** The authorized surface is the generalized helper plus the three hoisted calls plus the three cap-gated updates. Nothing else in `crates/maos-kernel-core/src/` may grow.
2. **The discriminating control lands WITH the guard** (Murat's catch — see AC3). The pin does not move until the rewritten control is proven red by deleting the predicate, then green when restored.
3. **Code first, then the pin.** Land the change, `cargo fmt --all`, re-measure the formatted tree, and re-pin to the **measured** value. 13.5d `:475`: an ahead-of-code re-pin disarms the exact-equality gate. `23234` is the measured expectation, not a licence to skip re-measuring.
4. **Exactly one literal moves:** `xtask/kernel-core-baseline.toml` `src_lines`. All three enforcement surfaces read it dynamically. **`xtask/fkcs-baseline.toml:5` (`src_lines = 23081`) is the FROZEN tag — do not touch it.**
5. **ADR-059 amended in the same commit** (Winston's rider), including scoping `:68`'s ZERO-Δ claim to 13.5b.
6. **AC4's seven stale pointers retired in the same commit** (John's rider) — a signed artifact calling Shared partitioned while gate output still says `CoverageGap` is internally untrue.
7. **No unrelated kernel growth rides along.** 13.5d `:473`: *"stop and re-escalate; do not spend the grant elsewhere."* This explicitly excludes the private-tier residue — see *Residuals*.

Also: add the **missing 13.5d HISTORY row** and this story's row to `xtask/kernel-core-baseline.toml` (see *Authorization record*).

**AC2 — `reject_principal_outside_private` guards all three Shared entry points, fail-closed and typed.**
- Implement exactly the shape in *Ratified implementation shape*. Winston's naming rider: the doc comment must tie the allowlist to the **principal index and the forget / subject-access paths**, so it is not misread as a generic privacy policy.
- **All three arms**, not just `write` — a partition that admits reads of a planted row is not a partition, and `SpiritMemoryView` exposes all three.
- ⚠ **Do not widen any enum.** `MemoryTier` is matched exhaustively with no `_` at `mod.rs:727-744`, `:801-804`, `:855-858`; `ForgetOutcome` (`:896-906`) and `CollectivePortError` (`:193-200`) likewise. `MemoryError` is free — no exhaustive production match exists in kernel-core. (`FrameKind` is `#[non_exhaustive]` at `transparency_log.rs:53-56` and `MemoryNamespace` has a wildcard arm at `private.rs:329-334`; 13.5b's Dev notes overstate both — correction folded in here.)
- Re-measure after `cargo fmt`; do not predict (Trap 2).

**AC3 — Null control #23 is converted into a real control, and falsified. This is the gating condition on the pin.**
> ⚠ **Murat's catch, and the single most important line in this story:** deleting Variant B's hoisted guard would **not** make the *current* test red, because it only ever writes `Coordination` at Shared. The existing `gdpr-backend-partition` leg is vacuous for Shared **regardless of the guard**. `check_reza_production_path`'s `run_test_leg` enforces `running 1 test` / `1 passed`, but it **cannot detect a semantically null assertion**. So the control must be rewritten *before* the guard can be considered proven — the fix and its detector land together or neither lands.

Rewrite the Shared discharge in `multi_backend_erasure_test.rs`:
- **(a)** Attempt a principal-shaped write **at the Shared tier's own entry point** and assert the typed partition refusal — mirroring how the Collective discharge now works at `:149-164`. Do the same for **read** and **scan**. Only then push `"shared"` into `proved_principal_empty`.
- **(b)** Scan `shared_memory` and assert **zero** principal-shaped rows, filtering on the **namespace column** (Trap 6).
- **(c)** **Keep the Coordination canary** — as its own separately named ADR-026 positive-retention assertion, *not* as the principal-empty discharge. The two claims must never share an assertion again.
- **(d)** **Falsify:** delete the hoisted predicate, confirm red *for that reason*, restore. Record both outputs.
- Assert the **typed** refusal (`MemoryError::NamespaceViolation`), never the diagnostic sentence.
- The disjoint-union invariant at `:166-187` keeps its shape; only the Shared input proof changes.

**AC4 — Every artifact that names Shared as an open gap is retired in the same commit.**
Seven live pointers — verified by `grep -rn '13-5h' crates/ xtask/ docs/`:
- `crates/maos-audit/src/erasure/regional_teardown.rs:57-71` — move `"shared"` from `UNCOVERED_STORES` into `REQUIRED_STORES`. The invariant `store_sets_partition_known_stores` (`:329-347`) exists to red if you move it in one set and not the other. `UNCOVERED_STORES` becomes empty — keep the constant and its doc. Its two doc comments naming 13-5h (`:62`, `:332`) update in the same edit.
- `crates/maos-bin/src/main.rs:8273-8276` — the `CategoryStatus::CoverageGap { reason: "…owner: Story 13-5h" }` category. **Decide and state which:** with the tier partitioned, `CategoryStatus::VerifiedEmpty` is the honest status (`proof.rs:22-26` — reuse, do not invent). Justify in ADR text; do not just swap the enum. Be honest about Trap 4 when you do.
- `crates/maos-bin/tests/erasure_uninstall_13_5b.rs:251` — `regional_uninstall_emits_erased_terminal_with_shared_coverage_gap` asserts `reason.contains("13-5h")`. It **will** red. Rewrite to the post-partition contract; keep the coverage it provides (region-pinned run still exits 0, still writes a teardown receipt).
- `xtask/src/check_reza_production_path.rs` — the `gdpr-regional-shared-coverage-gap` leg follows that test; rename and re-point it.
- Docs: `docs/adr/ADR-059-…md:22`, `:74`, `:101` (Residual 2 row) and `docs/runbooks/dr-1-restore-drill.md:134`, plus the **Reading an erasure artifact** table's Shared line.
- **Verification step, not optional:** afterwards `grep -rn '13-5h' crates/ xtask/ docs/` must return **zero** hits outside this story file and ADR-059's historical record.

**AC5 — Gate and CI legs are real controls on the existing host; no new gate.**
- **Fold onto `check-reza-production-path`** — already the v2.2-blocking GDPR host, already runs `multi_backend_erasure_test` as the `gdpr-backend-partition` leg (`:355-367`). `check-multi-tenant-loom` has no GDPR leg and `ABSENT_SUCCESSORS = &[]`; not the host. A new gate is ~13 registration surfaces for zero gain.
- Add a hermetic **`Blocking`** leg for the AC3 refusal test (TempDir + SQLite, no `#[ignore]`d Postgres — `Blocking` is correct per `gate_common.rs:84-101`).
- ⚠ **Anti-vacuity:** exactly one `#[test]` per leg, invoked by name with `--exact`; the transcript must show `running 1 test` **and** `1 passed`.
- Murat's condition: the Reza leg must be run **with the rewritten test selected**, so its non-vacuity check covers the *repaired* control.
- CI: `discipline.yml:2752-2753` already runs `multi_backend_erasure_test` whole-file, so no new step if the test stays in that target. New target → house idiom `cargo test --locked -p <crate> --test <name>`. ⚠ `cargo test --workspace` appears **nowhere** in `discipline.yml` — every suite must be named to run.
- **`ABSENT_SUCCESSORS` needs no edit:** `check_reza_production_path.rs:16-21` holds only `"11.4b …"` and `"13.6 …"`. 13-5h was never added. Confirm, don't assume.

---

## Authorization record — FLAG-Winston, 2026-07-25

**Granted by the operator on a unanimous 4/4 panel vote** (`VOTE: variant=B grant=approve extraction=waive headroom=accept`), following the precedent at `11-4a-enterprise-pdp-integration.md:359`.

| Panelist | Variant | Grant | Extraction | Headroom |
|---|---|---|---|---|
| Winston 🏗️ architect | B | approve | waive | accept |
| Murat 🧪 test architect | B | approve | waive | accept |
| John 📋 product | B | approve | waive | accept |
| Amelia 💻 engineering | B | approve | waive | accept |

**Authorized delta:** `src_lines` **23228 → 23234 (+6 physical)**, `maos-kernel-core` tokei **17890 → 17894 (+4)**.
**KLOC ceiling at the time of the grant: unchanged at 17 900**, which Variant B fit with 6 lines to spare — the grant deliberately did *not* depend on a ceiling move. Per Winston at the time: *"Raising a ceiling simply because the remaining margin is uncomfortable would invert the stated ADR-038 discipline."* **Superseded as capacity, not as authorization:** on 2026-07-25 the founder ratified replacing the tight-residual ceiling policy outright (`ceiling = measured + max(100, ceil(0.02 × measured))`, unanimous 5/5 panel), moving `maos-kernel-core` to **18 248** and flipping `kloc-check` from advisory to **BLOCKING**. This story's authorized kernel delta is still exactly +6; the extra capacity may not be spent here.

**`kloc.toml:42-45` extraction precondition — WAIVED, story-specifically.** The rule's own stated enforcement premise (`:45`, *"The gate's continuing failure on every CI run keeps the breach visible"*) is dead: it was written when kernel-core was ~21 370 against a 6 000 ceiling, and the Epic-10/Epic-12 retros reset ceilings to the tight measured residual — the crate is now **green**. This is a waiver, **not a repeal**: no unrelated kernel growth rides with it, and any over-ceiling follow-on re-escalates.

**Extraction candidate, measured and recorded for when it is genuinely needed:** `crates/maos-kernel-core/src/hot_swap/` is **1 809 physical lines with only 2 inbound refs** (`api.rs` re-export, `lifecycle/upgrade.rs`) — the least-called large module. ⚠ **But inbound is only half the measurement.** Its *outbound* coupling is **19 references across 6 kernel subsystems** (`scheduler`×8, `halt`×4, `iac`×3, `journal`×2, `telemetry`, `capability`). It is easy to stop calling and hard to move: extracting it verbatim would be **LOC laundering, not decomposition** (Winston, 2026-07-25). It remains the best Phase-4 candidate, but only as a *service boundary with explicit ports and composition-root wiring* — `memory` (3 152 / 19 inbound) and `scheduler` (2 848 / 23 inbound) are worse on both axes. **Extraction must never gate a correctness repair.**

**HISTORY rows to write when the pin moves:**
- the **missing 13.5d row** — `23202 → 23228 (+26)`, authorized on this branch lineage, never recorded (HISTORY currently ends at the J1 `23202` entry, `:248-263`);
- this story's row — `23228 → 23234 (+6)`, Variant B, GDPR Art.15/17 Shared-tier principal partition, operator-ratified 2026-07-25, unanimous panel.

---

## Traps

**Trap 1 — `REGISTERED_ERASURE_BACKENDS` has ONE character of headroom.**
`mod.rs:35` is **99 characters** and `max_width = 100` with no `rustfmt.toml`. Any rename or addition longer than one character wraps the line into ~6 and silently adds kernel lines *on top of* the grant. This story should not touch it — **do not rename anything on that line.**

**Trap 2 — the two budgets are measured differently; only one moves.**
`check-kernel-baseline` counts **every physical `.rs` line** under `crates/maos-kernel-core/src` — comments, docs, blanks, `#[cfg(test)]`, untracked files (`check_kernel_baseline.rs:94-105`) — and demands exact equality. `kloc-check` runs **tokei code-only** with `tests`/`benches`/`examples` excluded (`kloc_check.rs:167-193`). Variant B is +6 physical but only +4 tokei. **Measure both; re-base only the pin.**

**Trap 3 — the guard must not break the three tier matches.**
`MemoryTier` is matched exhaustively with no `_` at all three sites. The hoisted guard sits *above* each match and leaves the arms alone — keep it that way.

**Trap 4 — the partition makes pre-existing rows unreachable; it does not erase them.**
After the change, reading a *pre-existing* Shared principal row returns a typed refusal instead of the value. There are none in any fixture today (D-3's verdict), but a deployed Host could theoretically hold one. **Take an explicit position in the ADR:** refusal is correct (fail-closed; the row was never legitimately writable), *and* AC4's `VerifiedEmpty` choice must be honest that "unreachable" ≠ "erased".

**Trap 5 — the Coordination canary must survive, under its own assertion.**
ADR-026's operational requirement (quoted at `multi_backend_erasure_test.rs:106-108`) is that legitimate cross-Spirit data survives a principal forget. AC3(c) keeps it — separately named. Merging it back into the principal-empty discharge recreates null control #23 exactly.

**Trap 6 — `shared_store_contains` has no namespace filter.**
`multi_backend_erasure_test.rs:64-86` scans every `shared_memory` value for a substring. That is precisely why the swapped-namespace canary still satisfied the old assertion. Any new scan must filter on the **namespace column**, or you rebuild the same null control with new words.

---

## Tasks / Subtasks

- [x] **Task 0 — record the grant (bookkeeping; the escalation is done).** (AC: 1)
  - [x] Re-verify pin == actual == 23228 (`git ls-files 'crates/maos-kernel-core/src/*.rs' 'crates/maos-kernel-core/src/**/*.rs' | xargs wc -l | tail -1` — **both** globs; the recursive one alone misses top-level files and reports 23073). Confirmed 23228; the recursive-only trap reproduced at 23073.
  - [x] Confirm `kloc-check` kernel-core baseline before any edit. Confirmed 17890, aggregate 133989.
- [x] **Task 1 — the guard (Variant B).** (AC: 2)
  - [x] Generalize the helper; hoist three calls; delete the three in-arm Collective calls; update the three cap-gated sites. All nine sites in one atomic edit; `MemoryTier` verified `Copy + Debug` first, so the hoist cannot move the value and `{tier:?}` is valid.
  - [x] `cargo fmt --all`; re-measure both budgets. Physical **23234 (+6)**, tokei **17894 (+4)** — both exactly as authorized.
- [x] **Task 2 — kill null control #23 (gates the pin).** (AC: 3)
  - [x] Refusal-at-entry-point discharge for write/read/scan + namespace-filtered zero-row scan.
  - [x] Coordination canary retained under its own ADR-026 assertion.
  - [x] **Falsify:** delete the hoisted predicate, confirm red for that reason, restore. Done **per leg**, not once: each of the three deletions redded the suite at *its own* assertion. Outputs recorded in Debug Log.
- [x] **Task 3 — move the pin, once Tasks 1-2 are green.** (AC: 1)
  - [x] Re-pin `xtask/kernel-core-baseline.toml` `src_lines` to the **measured** value; touch nothing else. `fkcs-baseline.toml` verified byte-untouched by `git diff`.
  - [x] Write the missing 13.5d HISTORY row **and** this story's row.
  - [x] Confirm `check-kernel-baseline`, the a2a-tcp guard and the fkcs oracle all pass. All three green at 23234.
- [x] **Task 4 — retire every "Shared is a gap" claim.** (AC: 4)
  - [x] Store-set move; proof category; test rewrite; leg rename; ADR + runbook; zero-hit grep.
- [x] **Task 5 — gate and CI.** (AC: 5)
  - [x] Blocking `--exact` leg; re-point `gdpr-regional-shared-coverage-gap`; confirm `ABSENT_SUCCESSORS` needs no edit. Two Blocking legs land; `ABSENT_SUCCESSORS` confirmed unchanged; both CI targets already run whole-file.
- [x] **Task 6 — ADR-059 amendment (same commit).** (AC: 1, 2, 4)
  - [x] Record the grant, the Trap 4 position, the fork-(ii) rejection, and scope `:68`'s ZERO-Δ claim to 13.5b. Close Residual 2. Do **not** open a new ADR.

### Review Findings

- [x] [Review][Patch] Check Shared residue before the no-principal early return [crates/maos-bin/src/main.rs:8214]
- [x] [Review][Patch] Fail non-regional uninstalls when Shared residue is present [crates/maos-bin/src/main.rs:8270]
- [x] [Review][Patch] Preserve the proof when Shared verification cannot run [crates/maos-bin/src/main.rs:8268]
- [x] [Review][Patch] Remove remaining `13-5h` hits outside ADR history [docs/runbooks/dr-1-restore-drill.md:134; xtask/kernel-core-baseline.toml:306]

---

## Dev notes

- **Reuse, do not invent.** The helper already exists — generalize it. `CategoryStatus::{Removed, VerifiedEmpty, CoverageGap}` (`proof.rs:22-26`) is the vocabulary. The disjoint-union invariant, the Reza leg model and the `--exact` anti-vacuity contract all exist. This story's new code is **6 kernel lines**; its value is making an existing control tell the truth.
- **Why the partition is the whole fix.** With `write` refused at the boundary no new principal row can enter Shared; with `read`/`scan` refused no pre-existing row can be served. The tier becomes principal-empty *by construction* — exactly the property `proved_principal_empty` claims and has never tested.
- **The Shared store is SQLite, not the private tier's hybrid.** `shared_memory` is a durable table with PK `(writer_spirit_pid, namespace, key)` (`shared.rs:16-27`), with none of the in-memory/on-disk split that produced the private-tier residue. If a future story ever needs Shared erasure it is a `DELETE … WHERE namespace = ?`, not a filesystem walk.
- **Stale figure, corrected.** 13.5b calls the Collective fail-closed block "12 lines"; measured it is **10** (`mod.rs:755-764`), 19 with its comments. Shared needs neither — only the hoisted guard call.
- **`for_spirit.rs` is why this is three sites.** `for_spirit` fuses the PID (`mod.rs:397-398`) but the tier stays caller-supplied on all three methods.

## Budget — measured 2026-07-25 at `5ccd862c`

| Crate | Measured | Ceiling | Headroom | This story |
|---|---:|---:|---:|---|
| **`maos-kernel-core`** | **17 890** | ~~17 900~~ → **18 248** | ~~10~~ → **358** | **+4 → 17 894 (354 spare under the new ceiling)** |
| `maos-audit` | 6 117 | 6 150 | 33 | AC3 test rewrite (tests excluded from kloc) |
| `maos-bin` | 14 150 | 14 200 | 50 | AC4 proof category |
| `xtask` | 30 911 | 30 950 | 39 | AC5 leg |
| `_aggregate_hardfail` | 133 989 | 134 150 | 161 | → 133 993 |

Physical baseline pin: `xtask/kernel-core-baseline.toml` = **23 228** → authorized **23 234**. Separate number from the KLOC table (physical vs tokei).

## Previous-story intelligence

- **13.5b (immediate predecessor, same subject, commit `5ccd862c`)** — established that a control naming a store is not a control covering it. Its two ratified decisions (ADR-059 Decisions 8 and 9) are the vocabulary here: attestation scope is what was *covered*, and an artifact must describe what actually happened. It left this story's seven "Shared is a gap" pointers.
- **13.5d** — the only prior kernel grant on this branch lineage (+26, re-pin 23202→23228). Its terms are AC1's conditions. Its HISTORY row was never written; Task 3 fixes that.
- **13.3b** — a Blocking leg passed because an appended invalid marker, not the property under test, satisfied its `assert_ne`. Found only when the author neutered the guard. AC3(d) is that discipline; Murat's catch is the same failure mode caught *before* it ships.
- **13.1** — handed 13.5b an untested load-bearing argument (F16). 13.5b discharged it for Collective; this story discharges the Shared half.

## Testing standards

- **Proven-red is the bar,** and per AC3 it gates the pin.
- **Falsify your own control** (13.3b): neuter the exact predicate, confirm red *for that reason*.
- **Test placement matters to the pin.** Unit tests inside `crates/maos-kernel-core/src/**` count toward the pin; `crates/maos-kernel-core/tests/` and `xtask/tests/` do not. Prefer the latter. `kloc-check` excludes `tests` entirely, so AC3's rewrite costs no ceiling.
- **Hermetic → `Blocking`.** Nothing here needs Postgres.
- **Anti-vacuity:** one `#[test]` per leg, `--exact`, transcript shows `running 1 test` and `1 passed`.

## Gate discipline (§A7 reflex)

Legs land on **`check-reza-production-path`**. It is a control for this story only because (a) deleting the hoisted predicate reds the *rewritten* AC3 leg, (b) moving `"shared"` in one store-set constant but not the other reds `store_sets_partition_known_stores`, and (c) the retired `gdpr-regional-shared-coverage-gap` leg is *replaced*, not merely deleted. Forbidden shortcuts, all previously used in this epic: leaving a string in `ABSENT_SUCCESSORS` in place of a leg; an `available_arm_tests`-only leg; naming `abi-diff` or `check-empty-kernel` as a sensor (both blind here — `abi_diff.rs:8` pins the `maos-spirit-abi` manifest and `:47` can never fail on additions).

## Residuals

1. **⚠ Private-tier filesystem residue — OPEN, HELD FOR ITS OWN GRANT, successor `13-5i`.** Operator-ratified 2026-07-25: schedule **immediately after 13-5h, before 13.6**; **not** authorized under this grant. `PrivateMemoryStore::forget_principal` (`private.rs:319-337`) derives its removal set exclusively from the in-memory map. `MemoryValue::Markdown` is filesystem-canonical and deliberately never cached (`:183-188`) and always spills (`:158-161`); any value over the 4 KiB threshold spills too and is lost from the map once the process exits, since `new` never hydrates (`:30-36`). So the `fs::remove_dir_all` at `:360` never runs: the bytes survive while the signed proof says `memory_namespace: Removed { count: 0 }` and subject access reports the principal gone. Pinned by `private_tier_markdown_survives_the_forget_cascade` (`crates/maos-bin/tests/erasure_uninstall_13_5b.rs`), bound as Blocking leg `gdpr-private-markdown-residue-pinned`; ADR-059 Decision 10 / Residual 8. Estimated **+35..55** kernel lines — which will **not** fit in the 6 lines left after this story, so 13-5i must either extract `hot_swap` or carry its own measured ceiling re-base. That is the "separately justified authorization" the panel demanded.
2. **Shared-tier erase (fork (ii))** — rejected here on an empty subject set. If a future story ever gives the Shared tier legitimate principal data, the partition must be reversed *and* namespace-aware erasure designed against ADR-026. Record the reversal condition in ADR-059 so the argument is not silently inherited.
3. **`ADR-026` has no literal cross-Spirit-survival sentence.** The requirement is operational, asserted only in a test comment (`multi_backend_erasure_test.rs:106-108`); the ADR says only that the principal namespace lives in the private tier (`:12`). Either amend ADR-026 to state the retention requirement, or stop citing it as the source. Ownerless and open.
4. **`maos-kernel-core` structural headroom — RESOLVED as capacity, still open as architecture.** The 2026-07-25 ceiling-policy replacement gives the crate 18 248 (358 reserve at measurement), so this story leaves **354** rather than 6. That removes the operational squeeze; it does not discharge the decomposition debt. The `hot_swap` extraction (1 809 lines; 2 inbound / **19 outbound across 6 subsystems**) is Phase 4 of the standing `kloc.toml` plan — real architectural work, not a line-count move. ⚠ Note the Epic-5 plan's arithmetic never closed: its named extractions total ~13 627 LOC against a crate then at ~21 370, so the stated ~5 400 residual target was never reachable by its own enumerated moves (Mary, 2026-07-25). Ownerless; belongs to the Epic-13 retrospective alongside ceiling policy.

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` (Amelia 💻, bmad-dev-story).

### Debug Log References

**Task 0 — baselines before any edit.** Pin `23228 == 23228`; both-glob measurement `23228 total`; the recursive-only glob reproduced the documented trap at `23073 total`. `kloc-check` kernel-core `17890`, aggregate `133989`. `fkcs-baseline.toml` FROZEN tag `23081`.

**Task 1 — budgets after `cargo fmt --all`.** Physical `23234 total` (+6, exactly the authorized figure). Tokei `17894` (+4). `cargo check -p maos-kernel-core` clean; `cargo test -p maos-kernel-core` 257 unit tests pass.

**Task 2 — AC3(d) falsification, run per leg rather than once.** Each deletion of a single hoisted `reject_principal_outside_private(tier, namespace)?;` call, with the other two left in place:

| Predicate deleted | Verdict | First failing assertion |
|---|---|---|
| write hoist | **RED** | `shared principal write must be refused` |
| read hoist | **RED** | `shared principal read must be refused` |
| scan hoist | **RED** | `shared principal scan must be refused` |
| all three restored | **GREEN** | `running 1 test` / `1 passed` |

Each leg reds at *its own* assertion, not a collateral one, so all three are independently load-bearing. `mod.rs` restored byte-identical (verified by string equality against the pre-mutation snapshot, backup at `/tmp/mod_rs_13_5h.bak`).

**Task 3 — pin enforcement surfaces at 23234.** `check-kernel-baseline: PASSED (23234 == 23234)`; `t12b_kernel_core_byte_identical_line_count` `1 passed`; `frozen_snapshot_and_current_kernel_baselines_are_independently_valid` `1 passed`. `git diff --stat xtask/fkcs-baseline.toml` empty.

**Task 4 — `VerifiedEmpty` proven non-vacuous.** The Trap 4 test plants a principal row in `shared_memory` by raw SQL and the producer reports it: *"shared tier holds 1 pre-partition principal row(s); the Story 13.5h partition makes them unreachable but there is no delete path, so they are NOT erased"* — the count is live, the status degrades, and the teardown receipt is withheld.

**Task 5 — gate legs, `--exact` anti-vacuity contract.** All three `binding: blocking`, `attempted: true`, `green: true`, `detail: "running 1 test; 1 passed"`: `gdpr-backend-partition`, `gdpr-regional-shared-verified-empty`, `gdpr-shared-pre-partition-residue-fail-closed`. Gate verdict `check-reza-production-path: PASSED` across 33 legs.

**Regression.** `cargo test --workspace --no-fail-fast`: **3502 passed / 0 failed / 92 ignored across 449 suites.** `cargo fmt --all -- --check` clean. `check-kernel-baseline`, `kloc-check`, `check-multi-tenant-loom`, `check-reza-production-path` all PASSED.

### Implementation Plan

Executed in the story's task order, because the order is a correctness constraint rather than convenience: the guard (Task 1) had to exist before the control could be falsified against it (Task 2), and the control had to discriminate before the pin was allowed to move (Task 3). Tasks 4-6 touch no kernel-core file, so the pin measured in Task 3 stayed valid to the end — re-verified at completion.

One design decision was taken beyond the literal instruction, and it is the substantive one. AC4 directed `CategoryStatus::VerifiedEmpty` for the Shared category and added *"do not just swap the enum."* A bare swap would have been indefensible: `VerifiedEmpty` carries no payload, so its entire meaning is *"we checked"*, and emitting it unconditionally would have rebuilt — inside the fix itself — exactly the assert-instead-of-prove null control this story exists to delete. It would also have been false on the Trap 4 host, which holds pre-partition rows that the partition renders unreachable but cannot erase.

So the status is **earned at runtime**: `maos_audit::shared_tier_principal_row_count` counts principal-namespaced rows filtering on the namespace column, and the producer emits `VerifiedEmpty` only on zero, degrading to a `CoverageGap` that states the count otherwise. `"shared"` is admitted to `stores_covered` on the same condition, so a Host with residue drives `completed` false and signs no teardown receipt — fail-closed. This required threading `memory_db_path` into the cascade: `SharedMemoryStore` is opened on the Host-wide memory artifact (`main.rs:2441`), which under active tenancy is a **different file** from the team-sharded `audit_db_path` the cascade already received. Reading the audit shard would have silently found no table and returned a vacuous zero.

The two behaviours are pinned against each other by paired tests, so neither can rot into an assertion.

### Completion Notes

All five ACs satisfied; all 12 subtasks checked; zero regressions.

- **AC1 — grant spent exactly as authorized.** +6 physical (23228 → 23234) and +4 tokei (17890 → 17894), matching the panel's measured figures to the line. Code landed first, tree formatted, then re-measured, then re-pinned (13.5d `:475`). Exactly one literal moved; the FROZEN fkcs tag is untouched. Both HISTORY rows written, including 13.5d's retroactive `23202 → 23228`. No unrelated kernel growth: `mod.rs:35` was not touched (Trap 1), and 13-5i was **not** absorbed.
- **AC2 — guard.** `reject_principal_outside_private(tier, namespace)` stated as a negation of the allowlist, so a future `MemoryTier` is principal-rejecting by default. Guards write, read and scan; the three cap-gated sites adopt it so they cannot become an alternate bypass. No enum widened; the three exhaustive `match tier` blocks are untouched because the guard sits above them (Trap 3).
- **AC3 — null control #23 is dead.** The discharge now asserts the typed `MemoryError::NamespaceViolation` at the Shared tier's own entry points and scans for zero principal rows filtering on the **namespace column** (Trap 6). The Coordination canary survives under its own separately named ADR-026 assertion (Trap 5). Falsified per leg.
- **AC4 — seven pointers retired.** Every surviving `13-5h` mention now reads as provenance ("closed by") or lives in ADR-059's historical record and the pin's audit trail. No artifact still calls Shared an open gap.
- **AC5 — real controls on the existing host.** Two Blocking legs on `check-reza-production-path`; no new gate; `ABSENT_SUCCESSORS` needed no edit, as predicted. Both CI targets already run whole-file, so no new CI step.

**Behavioural change worth flagging at review.** A region-pinned Host carrying pre-partition Shared principal residue now fails its uninstall (non-zero exit, no teardown receipt) instead of silently attesting success. That is the intended fail-closed reading of Trap 4 and is documented in ADR-059 Decision 11 and the DR-1 runbook, but it is a genuine behaviour change for a hypothetical upgraded Host. No fixture, example or production path in the repository produces such a row, so nothing in-tree exercises it except the test that plants one deliberately.

**Residual 2 is closed.** Residual 1 (13-5i) remains open and explicitly unfunded by this grant.

### File List

| File | Change |
|---|---|
| `crates/maos-kernel-core/src/memory/mod.rs` | Generalized the helper to `reject_principal_outside_private(tier, namespace)`; hoisted one call above each of the three `match tier` blocks; deleted the three in-arm collective calls; re-pointed the three cap-gated sites. **+6 physical / +4 tokei — the entire authorized kernel delta.** |
| `crates/maos-audit/src/lib.rs` | Added `shared_tier_principal_row_count` — namespace-column-filtered count that makes `VerifiedEmpty` earned. |
| `crates/maos-audit/src/erasure/regional_teardown.rs` | Moved `"shared"` from `UNCOVERED_STORES` into `REQUIRED_STORES` (now empty, retained deliberately); rewrote both store-set tests; added the fail-closed residue case. |
| `crates/maos-audit/tests/multi_backend_erasure_test.rs` | Killed null control #23: typed refusal at Shared write/read/scan, namespace-filtered zero-row scan, Coordination canary split into its own ADR-026 assertion. |
| `crates/maos-bin/src/main.rs` | Threaded `memory_db_path` into the uninstall cascade; Shared category now `VerifiedEmpty`-on-verified / `CoverageGap`-on-residue, gating `"shared"` in `stores_covered`. |
| `crates/maos-bin/tests/erasure_uninstall_13_5b.rs` | Rewrote the coverage-gap test to the post-partition contract; added `plant_pre_partition_shared_row` and the Trap 4 fail-closed test. |
| `xtask/src/check_reza_production_path.rs` | Renamed the retired leg to `gdpr-regional-shared-verified-empty`; added `gdpr-shared-pre-partition-residue-fail-closed`; documented the refusal coverage on `gdpr-backend-partition`. |
| `xtask/kernel-core-baseline.toml` | `src_lines` 23228 → **23234**; converted the staged authorization block into a landed HISTORY row with the per-leg falsification evidence. |
| `docs/adr/ADR-059-operator-authority-collective-erasure.md` | Decision 1/8 superseded notes; Decision 11 grant marked spent with falsification evidence; explicit unreachable-vs-erased position; Residual 2 → CLOSED. |
| `docs/runbooks/dr-1-restore-drill.md` | Shared is no longer a gap; documented the residue escalation path and the updated `stores_covered` set. |
| `_bmad-output/implementation-artifacts/13-5h-shared-tier-principal-partition.md` | Tasks and review findings checked; Dev Agent Record, File List and Change Log completed; Status → done. |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | 13-5h `ready-for-dev` → `in-progress` → `review` → `done`. |

### Change Log

| Date | Change |
|---|---|
| 2026-07-25 | Story created from the 13.5b preflight D13 / Residual 2 and the 13.5b code review. Fork (i) vs (ii) **settled by evidence** (no `(Shared, Principal)` caller exists anywhere in the workspace); null control #23 **empirically proven** by swapping the canary namespace and observing the discharge stay green. |
| 2026-07-25 | **Kernel grant AUTHORIZED.** Both implementation shapes built, compiled, formatted, tested and measured against the real tree, then reverted: Variant A **+26 physical / +20 tokei (ceiling BREACH)**, Variant B **+6 / +4 (fits)**. Panel voted **4/4 unanimous** `variant=B grant=approve extraction=waive headroom=accept`; operator ratified. Story re-specified to Variant B; the second budget (KLOC ceiling) is **no longer required to move**. Murat's condition added as the gating constraint on the pin: the current control would **not** red on guard deletion, so AC3's rewrite must land with the guard. `hot_swap` recorded as the measured extraction candidate for the next story that cannot fit. 13-5i held for its own grant. |
| 2026-07-25 | **IMPLEMENTED.** Variant B landed in nine sites; both budgets measured on the formatted tree at exactly the authorized figures (+6 physical → 23234, +4 tokei → 17894). Null control #23 replaced by a discriminating one and falsified **per leg** — deleting the write, read or scan hoist each reds the suite at its own assertion — before the pin moved. Both HISTORY rows written, including 13.5d's retroactive `23202 → 23228`. Seven stale pointers retired; `UNCOVERED_STORES` now empty. **Design decision beyond the literal AC4 instruction:** `CategoryStatus::VerifiedEmpty` is EARNED at runtime by `shared_tier_principal_row_count` rather than asserted — a bare enum swap would have rebuilt the null control inside the fix and lied on the Trap 4 host, so residue degrades to `CoverageGap`, withholds `"shared"` from `stores_covered`, and refuses the teardown receipt (fail-closed). Required threading `memory_db_path`, since the shared store lives on the Host-wide artifact, not the team-sharded audit DB. Two Blocking legs added to `check-reza-production-path` (33 legs, PASSED). Regression: **3502 passed / 0 failed / 92 ignored across 449 suites**, `cargo fmt --check` clean, all four gates green. |
| 2026-07-26 | **CODE REVIEW COMPLETE.** Three parallel adversarial layers produced four actionable findings; all four patched. Shared residue verification now runs before destructive erasure and before the empty-principal early return; Shared residue fails every deployment shape after persisting a signed partial proof; non-regional and Shared-only upgrade states have dedicated regression tests. Exact `13-5h` references outside the story and ADR-059 history were removed. Targeted uninstall tests, the backend partition control, `check-reza-production-path`, `kloc-check`, formatting and Rust diagnostics are green. |
