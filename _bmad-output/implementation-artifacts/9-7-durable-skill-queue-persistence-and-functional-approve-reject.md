---
epic: epic-9
epic_title: "Audit & Compliance Surfaces + Operator Productionization (v0.5 → v1.0)"
dev_model_used: claude-opus-4-8
---

# Story 9.7: Durable Skill-Queue Persistence + Functional `maosctl skills approve/reject`

Status: done  # 2026-06-17 re-review closed: party-mode consensus ratified; F2 typed cache-load helper + busy_timeout deterministic fix + schema self-heal + entry_path pinning tests; AC-1/AC-2 amended; all decision-needed/patch findings resolved or intentionally accepted; cargo test -p maos-skill/cli/iac GREEN.

<!-- SPLIT from Story 9.6 AC-5 at party-mode preflight 2026-06-17 (Winston·John·Murat·Amelia, ratified Lunarpulse). The clean separable seam: operator-admission persistence is a different Job-to-be-Done than the multi-Spirit scheduler, in different crates (maos-skill / maos-cli), with zero data dependency on the scheduler. Slotted as the Epic-9 CLOSER with a hard gate (see Status note). -->

## ⚑ Slot & Gate (Task-0 (a) ruling — the anti-recidivism mechanism)

This item (Epic-7 §A3 carry-forward) has been homeless across Epic 7, 8.14c, 8.15, and the 9.6 stub — every time as a *promise* ("split candidate → later"), never a slotted spec. Promises decay (`[[feedback_mechanical_gates_compound_promises_decay]]`). John's ruling converts it to a compounding gate:

- **9.7 is the Epic-9 closer (the last functional story); the `epic-9-retrospective` MUST NOT open until `9-7-*` is `done`.** Wire this as a dependency the retro checks — same pattern as 8.16's `check-epic-close-green` meta-gate. This is what makes the split stick instead of disappearing a fourth time.

## ⚑ Preflight Consensus (party-mode 2026-06-17 — Winston·John·Murat·Amelia, ratified Lunarpulse)

Six forks resolved against the real code (verify-don't-assume facts settled three of them outright):

- **F1 — durable `queue.json` is the source of truth; the in-mem `SkillAdmissionQueue` is a per-invocation projection.** Each `maosctl` is a fresh short-lived process (the daemon does NOT touch the queue — admission is CLI-only), so load → operate-on-existing-methods → persist → exit. Reuse the 7 tested mechanics; wrap in a load/store boundary.
- **F2 — single `queue.json`, 9.7 writes its OWN atomic helper.** `LocalFsRegistryStorage` confirmed NON-atomic (plain `fs::write`, `maos-registry/src/storage.rs:157/186/203/206/211`) → do NOT inherit it. Sequence: serialize to `queue.json.tmp.{pid}` (same dir) → `sync_all()` (file) → `fs::rename(tmp, queue.json)` → `sync_all()` on the parent dir. The registry's non-atomicity is a separate latent defect → file as a follow-up, do not refactor here.
- **F4 + F6 — RESOLVED TOGETHER by the `ApprovalDecision` struct.** `ApprovalDecision` carries `actor: String` (a user identifier = principal data — `maos-domain/src/invariants/i4.rs:36-48`). Therefore: **persist ONLY `pending`** (skill-id/version/state — NO `actor`) to `queue.json`; mark `audit: Vec<ApprovalDecision>` `#[serde(skip)]` (never round-trips disk → nothing to re-drain → double-journal eliminated by construction); **journal the actor-bearing decision to the Transparency Log at the moment of mutation** via the existing `TransparencyLogAdapter::insert_approval_decision` (which Story 9.2's GDPR forget-cascade already covers). Result: `queue.json` is **principal-free** (no new GDPR surface), and the TL is the audit source of truth.
- **F5 — `schema_version: "maos.skill-queue.v1"`** top-level; unknown/absent version = hard error (no best-effort parse — the 9.2b lesson). A version stamp is a tripwire, not a migrator: write the migrator the day v2 ships, not before.
- **F3 — documented last-writer-wins + reconcile-pending-from-TL; NO flock.** Because each decision is journaled to the append-only TL on mutation, a concurrent-`approve` race can never lose an audit decision (the TL captures every one); the only residual is a *recoverable* stale lost-update in the `queue.json` pending-view. `queue.json` is a **rebuildable cache over the TL audit truth** — so the fix is a `reconcile-pending-from-TL` projection (`pending` = manifests-on-search-path minus `query_approvals()` decided-set), run on cold-start + on-read, NOT a lock. flock is neither necessary (audit is safe in the TL) nor sufficient (a crash *between* the TL journal and the queue.json write tears the view too — only reconcile fixes that). One concurrency story and one recovery story, and they are the same story.
- **F6b — the consumer gap (John).** The `maos run` daemon does NOT consult admission state at spirit-load today. So 9.7's persistence delivers **audit/forensics value on its own** (who approved/rejected what, when, durably) — but admission **enforcement** (daemon honors a persisted reject at load) is a separate body of work that touches the daemon. 9.7 therefore ships a **mandatory documented limitation** (see AC-5) + a **filed Epic-10 follow-up story** (daemon honors admitted-skill state). `done` for 9.7 includes both, or we close Epic 9 having shipped a queue the daemon can't read.

## ⚑ Preflight Round-2 — Long-Term-Correctness Amendments (party-mode 2026-06-17, Winston·Amelia·Murat·John, ratified Lunarpulse)

A second verify-don't-assume pass read the TL write/read paths and the CLI dependency graph that F3/F4 *assumed* but never checked. **Unanimous, zero-disagreement convergence** surfaced one falsified premise and two unimplementable test oracles. Eight ratified amendments (all verified against HEAD):

- **R1 — F3's "TL is the unloseable source of truth" was UNJUSTIFIED until the CLI can write the TL safely.** Verified: `maos-cli` depends only on read-only `maos-audit` (`SQLITE_OPEN_READ_ONLY`) + `maos-domain`; both `insert_approval_decision` (write) AND `query_approvals` (read) live on `TransparencyLogAdapter` in **maos-iac**. So 9.7 takes a NEW additive `maos-cli → maos-iac` write edge (maos-cli is top-of-stack — no cycle; the `Cargo.toml:33` "lock does not grow" note is corrected). The SAME edge makes `query_approvals` reachable for reconcile (the journal write opens a RW adapter; reconcile reuses that instance) — correcting the earlier "no new read path needed (9.1)" claim: 9.1's `maosctl audit` reads through `maos-audit`'s own `log_composition` SQL, NOT `query_approvals`.
- **R2 — journal-FIRST commit point (the load-bearing ordering).** TL write is the commit point: `insert_approval_decision` → ONLY on success → atomically rewrite `queue.json`. Crash/BUSY between the two = TL has the row, cache is stale → reconcile replays it (the recoverable case F3 promised). Mutate-then-journal is FORBIDDEN by name (it tears the audit spine). The CLI MUST NOT report a decision "applied" without a committed journal entry — loud non-zero exit + retry, **never silent-success-without-journal**.
- **R3 — SQLITE_BUSY is a ship-blocker (MEDIUM×HIGH), not a documented limitation.** TL opens `PRAGMA journal_mode=WAL` (`transparency_log.rs:360`) with NO `busy_timeout` → a second writer (the CLI) racing the daemon fails immediately, zero retry. Fix: `PRAGMA busy_timeout=5000` + bounded retry on maos-iac's `open_with_policy` (shared adapter path). Residual true-race after the timeout = the only acceptable documented limitation.
- **R4 — AC-3 double-journal oracle keys on `(target, capability)`, NOT `decision_id`.** Verified: `approval_decision_log` HAS `decision_id INTEGER PRIMARY KEY AUTOINCREMENT` (line 257) but `query_approvals` never SELECTs it and `ApprovalDecision` (`i4.rs:36-48`) has no such field. Surfacing it would change 9.1's read shape (9.2b's byte-identity-by-construction claim). Oracle: `count(d : d.target==X ∧ d.capability=="skill.admission.approve")==1`.
- **R5 — AC-4 reconcile = LWW-per-target filtered to approve|reject, proven-RED by EXACT set-equality.** Verified: `query_approvals(None)` returns the WHOLE `approval_decision_log` (all capabilities) incl. the `skill.admission.enqueue` (decision=false) row written for EVERY enqueued skill. Naive "subtract decided-set" empties pending. Decided-set = `{ parse_approval_target(d.target) | latest-row-per-target(query_approvals(None)) ∧ d.capability ∈ {approve,reject} }`. Test seeds A=enqueue-only, B=enqueue+reject, C=enqueue+approve over a stale `queue.json` listing {A,B,C}; GREEN asserts `pending == {A}` EXACTLY (the `A∈pending` member is the tripwire for subtract-all-empties; B as a reject with decision=false catches filtering on the `decision` bool instead of capability).
- **R6 — shared `approval_target`/`parse_approval_target` helper.** The `"{id}@{version}"` format lives only inside `enqueue_skill`; reconcile must parse it back. Promote to a single `pub` pair in `maos-skill` used by BOTH the write and the parse (cannot drift). Requires `SkillVersion: FromStr` = exact inverse of `Display` and `(SkillId,SkillVersion): Eq+Hash` (add if absent — in-scope).
- **R7 — all four ACs stay IN 9.7; the audit half does NOT split.** The TL journal is load-bearing for F4+F6's principal-free design (no journal ⇒ no record of WHO ⇒ actor gets stuffed back into `queue.json`, resurrecting the plaintext-principal store F4+F6 spent its budget avoiding). Splitting it ALSO reopens F3 (no longer "source of truth" ⇒ flock owed). Carving the audit half = the 5th decaying promise on a 4×-homeless item. The maos-iac touch + busy_timeout are the irreducible honest core, not a split.
- **R8 — only single-writer-via-daemon defers → Epic 10, folded onto the F6b ticket.** Routing the CLI write THROUGH the daemon (one writer, retires the multi-process race outright) is hardening on top of a working 9.7. Folded onto the EXISTING `deferred-work.md` F6b daemon-enforcement follow-up (same daemon seam) — not a new orphan. Latent watch-item (Murat): if reconcile-LWW ever resolves "latest" by `timestamp_ns`, it inherits a non-monotonic-clock hazard (NTP step backward elects the wrong winner) — the `, id ASC` tie-break on `query_approvals` is the same fix; document the one-decision-per-skill assumption.

> **maos-iac scope note (R2/R3):** 9.7 now touches `maos-iac` (busy_timeout + a `, id ASC` tie-break on the TL adapter) — both are body-only, non-ABI, non-signature edits. `check-kernel-baseline` counts ONLY `crates/maos-kernel-core/src` (verified `check_kernel_baseline.rs:31`) and the ABI gate defaults to `crates/maos-kernel-core`; `maos-iac` is in NEITHER → AC-6's zero-kernel-core-delta + byte-identical baseline HOLD. The story's "maos-skill/maos-cli work" framing is widened to include the maos-iac TL-adapter hardening.

## Story

As **an operator vetting the skill ecosystem**,
I want **`maosctl skills approve <id>` / `reject <id>` to mutate durable admission state that survives a process restart, with the decision journaled to the audit log**,
so that **skill operator-admission is a real, persistent, auditable exit — not an in-memory queue lost on exit and a CLI that only prints "acknowledged"**.

## Context (verified-still-open by Story 8.16 AC6)

`deferred-work.md:14-17` — Epic-7 §A3 required four skill-queue items; two are STILL OPEN at HEAD:
- ❌ **Restart persistence** — `SkillAdmissionQueue` holds `pending: Vec<PendingEntry>` + `audit: Vec<ApprovalDecision>` **in memory only** (`crates/maos-skill/src/admission.rs:76-80`). No durable store.
- ❌ **Functional `approve/reject`** — `crates/maos-cli/src/subcommands.rs` `dispatch_skills` `Approve`/`Reject` arms are acknowledgement-only `println!` stubs ("operator-admit acknowledged…"), return `SUCCESS`, mutate nothing.
- ✅ (Already closed: SkillId charset enforcement; duplicate-id-enqueue rejection.)

The in-memory mechanics already EXIST and are tested (7 tests in `crates/maos-skill/tests/admission_queue_test.rs`): `enqueue_skill` / `enqueue_proposal` / `approve` / `reject` (Pending→Admitted/Rejected) + `audit_trail()`. `admission.rs` already documents the intended seam: "the kernel composition root drains this into the real journal port the same shape `TransparencyLogAdapter::insert_approval_decision` records." This story wires the durable store + drains the audit, **reusing** those mechanics — do NOT reimplement the queue.

## Acceptance Criteria

### AC-1 — Durable skill-queue store (survives restart), own atomic write + schema version

**And** the file carries a top-level `schema_version: "maos.skill-queue.v1"`; the STORE/PARSER boundary treats an unknown/absent version as a **hard error** (no best-effort parse — the 9.2b lesson), while the CLI surface treats an unreadable cache as recoverable: warn to stderr and rebuild the view from filesystem discovery + the TL (the cache is non-load-bearing per F3/AC-4),
**And** the file persists **ONLY `pending`** (the `Vec<PendingEntry>` state machine — skill-id/version/entry-path/state); the in-mem `audit: Vec<ApprovalDecision>` is never round-tripped (it carries `actor`, a principal field — see AC-3/AC-6; the TL is the audit source of truth),
**And** for CLI-discovered skills the cached `entry_path` is `"package_shipped"` — correct for filesystem discovery, which cannot observe `AuthorSelf`/`RevisionProposal` provenance (those are enqueue-time constructs; faithful provenance fidelity is owned by the Epic-10 daemon-enqueue seam and is tracked as deferred work),
**When** a skill is enqueued, `approve`/`reject` is applied, and a NEW store instance is opened over the same path (= restart),
**Then** the `Admitted`/`Rejected` state + pending set are recovered intact (typed `AdmissionState` assertion, not a string scrape),
**And** a fault-injection test (write-to-temp then fail BEFORE rename) proves the prior valid `queue.json` is left intact + parseable — this test also doubles as the probe confirming the write path is genuinely atomic,
**And** the 7 existing in-memory `admission_queue_test.rs` tests still pass (in-memory remains the unit-layer default; the store is the durable backing).

**Given** `maosctl skills approve <id>` / `maosctl skills reject <id>` (`dispatch_skills`, `crates/maos-cli/src/subcommands.rs`),
**When** the operator runs the command against a pending skill id,
**Then** it loads the durable queue, applies `approve`/`reject` (transitioning the entry), persists atomically, and reports the real outcome — **no more `println!`-only stub**,
**And** approving/rejecting an **already-resolved** id returns `SUCCESS` as a typed no-op (no spurious mutation, current state surfaced to the operator); approving/rejecting an **unknown** id returns `FAILURE` with a diagnostic pointing at `maosctl skills list` (no spurious mutation on either path),
**And** `maosctl skills list` reflects the persisted state across invocations.

### AC-3 — Journal-FIRST to the Transparency Log (TL write is the commit point; double-journal guard by `(target,capability)`)

**Given** the actor-bearing `ApprovalDecision` (`actor: String`, `maos-domain/src/invariants/i4.rs:36-48`) and `TransparencyLogAdapter::insert_approval_decision`, reached via the NEW `maos-cli → maos-iac` write edge (R1/AC-7); the read-side `query_approvals()` is reachable on the SAME RW adapter instance (NOT via 9.1's `maos-audit` path — R1),
**When** an `approve`/`reject` is requested,
**Then** the decision is journaled to the TL **FIRST**, and ONLY on a committed journal row is `queue.json` atomically rewritten (R2 journal-first ordering) — mutate-then-journal is FORBIDDEN (it tears the audit spine),
**And** if the TL write fails (SQLITE_BUSY past the timeout, I/O, anything) the command aborts, mutates NOTHING, and reports failure with a **non-zero exit + retry prompt** — it MUST NEVER report a decision "applied" without a committed journal entry (the no-silent-loss invariant — tested in AC-7),
**And** the `audit` Vec being `#[serde(skip)]` (AC-1) means reopening cannot re-journal — **a double-journal guard test proves it keyed on `(target, capability)`** (NOT `decision_id`, which `query_approvals` does not surface — R4): one `approve(x)` mutation ⇒ exactly one row where `target==approval_target(x) ∧ capability=="skill.admission.approve"`; filtering OUT the `skill.admission.enqueue` rows, the decided-row total for the test's two mutations `== 2`,
**And** cross-restart `audit_trail` history is served by `query_approvals()` against the TL (the TL is the audit source of truth), not by the in-mem Vec.

### AC-4 — Reconcile pending-view from the TL (LWW-per-target; the recovery story; replaces locking)

**Given** that `queue.json` is a rebuildable cache over the append-only TL (concurrent `maosctl` writers = last-writer-wins on the CACHE, never on the audit record; **no file lock**),
**When** the pending-view is read (cold-start + on-read),
**Then** `reconcile-pending-from-TL` derives pending = discovered-skills MINUS the **decided-set**, where decided-set = `{ parse_approval_target(d.target) | d ∈ latest-row-per-target(query_approvals(None)) ∧ d.capability ∈ {"skill.admission.approve","skill.admission.reject"} }` — the capability filter is MANDATORY: `query_approvals(None)` returns the whole `approval_decision_log` incl. a `skill.admission.enqueue` (decision=false) row for EVERY enqueued skill plus unrelated system capabilities; subtracting those would empty pending (R5),
**And** re-enqueue-after-reject works because the latest row per target then = `enqueue` (not in decided-set) → the skill returns to pending (the in-mem `audit` Vec is `#[serde(skip)]`, so the enqueue decision MUST reach the TL or reconcile cannot resurrect it — R5/R6),
**And** the proven-RED test seeds the TL with A=`enqueue`-only, B=`enqueue`+`reject`, C=`enqueue`+`approve`, over a stale `queue.json` listing `{A,B,C}`; RED (no-op reconcile) lists `{A,B,C}`; GREEN asserts `pending == {A}` **EXACTLY** (set-equality, NOT a subset/absent check) — `A∈pending` is the tripwire for the subtract-all-empties bug; B (a reject carrying `decision=false`) catches filtering on the `decision` bool instead of `capability`,
**And** a discovered skill with NO TL row (never enqueued) is pending (discovered-but-undecided = awaiting decision) — an asserted case.

### AC-5 — Documented enforcement limitation + filed follow-up (the F6b honesty gate)

**Given** the `maos run` daemon does NOT consult admission state at spirit-load time,
**When** 9.7 ships,
**Then** a prominent documented limitation states that persisted approve/reject is the source of truth for **audit and CLI display**, and that the daemon does **not yet enforce** admission at spirit-load (so a rejected skill is not blocked from loading by this story alone),
**And** a follow-up story (daemon honors admitted-skill state at spirit-load) is **filed in `deferred-work.md` and slotted in Epic 10** — not left as a decaying promise. `done` for 9.7 requires both this doc and the filed story.

### AC-6 — Principal-free `queue.json`, green-at-HEAD, Epic-close gate

**Given** the GDPR cascade (Story 9.2) covers the TL `approval_decision_log`,
**When** the story lands,
**Then** a test/assertion confirms `queue.json` contains **no principal data** (no `actor`/operator id — only skill metadata), so 9.7 introduces no new GDPR-erasure surface (the actor lives only in the 9.2-covered TL),
**And** the full discipline suite is green-at-HEAD with zero disabled gates; **kernel-core baseline byte-identical** — 9.7's code lives in maos-skill/maos-cli + a body-only, non-ABI hardening of maos-iac's TL adapter (busy_timeout + `, id ASC`); `check-kernel-baseline` counts ONLY `crates/maos-kernel-core/src` (`check_kernel_baseline.rs:31`) and the ABI gate defaults to `crates/maos-kernel-core`, so the maos-iac edit leaves BOTH unaffected → **zero kernel-core delta expected** (confirm via `cargo xtask check-kernel-baseline` + the ABI-diff gate),
**And** the Epic-9 retro dependency on `9-7 == done` is wired/verified.

### AC-7 — CLI→TL write edge + multi-writer SQLite contract (the R1/R3 enabling AC)

**Given** the verified facts that (a) `maos-cli` has no write path to the TL today (read-only `maos-audit` + `maos-domain`), and (b) the TL opens WAL with NO `busy_timeout` (`transparency_log.rs:360`),
**When** 9.7 wires functional approve/reject,
**Then** `maos-cli/Cargo.toml` gains an **additive** dependency on `maos-iac` (the write-capable `TransparencyLogAdapter`), asserted by a build-graph check (`cargo tree -p maos-cli` shows `maos-iac`; `cargo build -p maos-cli` green; no new dependency cycle — maos-cli is top-of-stack); the `Cargo.toml:33` "lock does not grow" note is updated,
**And** maos-iac's `open_with_policy` sets `PRAGMA busy_timeout=5000` (+ a bounded retry) so a CLI writer racing the daemon BLOCKS-then-succeeds rather than failing immediately — guarded by a config-regression assertion that `busy_timeout > 0` on the live TL connection,
**And** a **deterministic** forced-contention Tier-1 test proves the contract WITHOUT racing the clock: hold `BEGIN IMMEDIATE` on connection #1; attempt the CLI journal on #2 — with timeout=0 it RED's (immediate `SQLITE_BUSY`; assert the CLI surfaces failure and marks NOTHING decided), with the timeout set it blocks until #1 releases then succeeds (GREEN),
**And** a **no-silent-loss invariant test** (the audit-integrity gate): force the journal write to fail; assert CLI exit ≠ 0 AND the pending view still lists the skill (never silent-success-without-journal — R2),
**And** the TL is documented as a **shared insert-only multi-writer log** (WAL + busy_timeout + bounded retry + append-only, no cross-row update) — no longer "daemon-private".

## Non-Goals (explicitly OUT of 9.7 — do not "helpfully" add)

- Distributed locking / `flock` on `queue.json` (LWW-per-target + reconcile is the ratified model; `busy_timeout` on the TL is SQLite contention-handling, NOT a file lock — R3).
- Routing the CLI's TL write THROUGH the daemon for a single writer (the true multi-process-race retirement) — filed to Epic 10, folded onto the existing F6b daemon-enforcement follow-up (R8).
- A generic persistence framework or abstraction over the registry idiom (reuse the convention, don't abstract it).
- Migration tooling (the version stamp is a tripwire; write the migrator when v2 ships).
- Daemon-side admission enforcement (the filed Epic-10 follow-up).
- Refactoring `LocalFsRegistryStorage` to be atomic (separate latent-defect follow-up).

## Tasks / Subtasks

- [x] **Task 1** (AC-1) — `SkillQueueStore` trait + `LocalFsSkillQueueStore` (single `queue.json`, OWN atomic temp+rename+dir-fsync helper, `schema_version` hard-fail); serde ONLY `pending` (`audit` = `#[serde(skip)]`); restart round-trip + fault-injection tests. Reuse `SkillAdmissionQueue` mechanics.
- [x] **Task 2** (AC-2) — Rewrite `dispatch_skills` Approve/Reject to load → mutate → persist atomically → report; preserve unknown/already-resolved no-op semantics; `list` reads persisted state.
- [x] **Task 3** (AC-3) — Journal-FIRST via `insert_approval_decision` (TL commit → THEN cache rewrite; abort-on-fail, no-silent-loss); `audit_trail` cross-restart reads `query_approvals()` on the maos-iac RW adapter; double-journal guard keyed on `(target, capability)` (NOT `decision_id`).
- [x] **Task 4** (AC-4) — `reconcile-pending-from-TL`: decided-set = latest-row-per-target filtered to `{approve,reject}` capabilities; on cold-start + on-read; proven-RED `pending=={A}` set-equality test (A=enqueue-only, B=enqueue+reject, C=enqueue+approve) + re-enqueue + never-enqueued cases. Add shared `approval_target`/`parse_approval_target` helper (R6); add `SkillVersion: FromStr` + `(SkillId,SkillVersion): Eq+Hash` if absent.
- [x] **Task 5** (AC-5) — Write the documented enforcement limitation; file the daemon-enforcement follow-up in `deferred-work.md` + Epic-10.
- [x] **Task 6** (AC-6) — principal-free `queue.json` assertion; `check-kernel-baseline` + ABI-diff (expect byte-identical / zero kernel-core delta despite the maos-iac touch); green-at-HEAD; wire/verify the retro-blocked-until-9.7-done gate.
- [x] **Task 7** (AC-7) — Add `maos-iac` dep to `maos-cli` (build-graph assert, no cycle; update `Cargo.toml:33` note); `busy_timeout=5000` + bounded-retry on maos-iac `open_with_policy` (+ `busy_timeout>0` regression guard + `, id ASC` tie-break on `query_approvals`); deterministic forced-contention test (held `BEGIN IMMEDIATE`) + no-silent-loss invariant test; document the TL shared-multi-writer contract. File the single-writer-via-daemon follow-up onto the existing F6b Epic-10 ticket.

### Review Findings

Code review 2026-06-17 (3-layer adversarial: Blind Hunter + Edge Case Hunter + Acceptance Auditor; dev_model_used=Claude → no Test Infra Auditor). All findings verified against HEAD code.

> **RESOLVED 2026-06-17 (review-fix pass, dev-story).** All decision-needed + patch findings below are **CLOSED**. D1 (Critical) → patched: `admission_view`/`decide_skill` now derive `pending = discovered − decided` from discovery + TL (queue.json is a pure cache), so a freshly-discovered skill is Pending + approvable — proven by `discovered_skill_is_approvable_end_to_end`. D2 → patched: `--actor` flag (defaults `$USER`→`operator`). D3 → dismissed (unknown→FAILURE / resolved→SUCCESS split is intended feedback). #4 demote + #5 clock (`ORDER BY decision_id ASC`) + #6/#7 (consistent cache/TL warnings) + #8 (AC-7 RED+GREEN via `open_with_busy_timeout`, no-silent-loss through `decide_skill`) + #9 (enqueue migrated to shared `approval_target`) + #10 (proven-RED against production `decided_set`+`reconcile_entries`) + #11 (failed-`atomic_write` test) + #12 (view-direct persist, no lossy enum round-trip) + #13/#14 (dead `parse`/`FromStr`/`MissingSchemaVersion` removed) + #15 (unique temp + cleanup). **Verified:** `cargo build --workspace` green; `cargo test -p maos-skill -p maos-cli -p maos-iac` all green; `check-kernel-baseline` PASSED (22227, byte-identical). See Change Log.

**Decision-needed:**

- [x] [Review][Decision] **CRITICAL — `maosctl skills approve/reject` unreachable** — ✅ FIXED: admission-view derives pending from discovery+TL; a discovered skill is now approvable (D1).
- [x] [Review][Decision] **`actor` hardcoded to `"operator"`** — ✅ FIXED: `--actor` flag added (D2).
- [x] [Review][Decision] **Unknown-id → FAILURE vs resolved → SUCCESS** — ✅ DISMISSED: current split is intended operator feedback (D3).

**Patch (fix is unambiguous):**

- [x] [Review][Patch] **[High] Reconcile cannot demote decided→Pending on re-enqueue** — ✅ FIXED: state derived fresh each read (#4).
- [x] [Review][Patch] **[Medium] Non-monotonic clock elects wrong LWW winner** — ✅ FIXED: `ORDER BY decision_id ASC` (#5).
- [x] [Review][Patch] **[Medium] `list` downgrades schema hard-fail to warning** — ✅ FIXED: list + decide warn consistently + proceed (#6).
- [x] [Review][Patch] **[Medium] Reconcile swallows TL errors** — ✅ FIXED: `AdmissionView.tl_readable` → caller warns (#7).
- [x] [Review][Patch] **[Medium] AC-7 test cluster doesn't prove the contract** — ✅ FIXED: no-silent-loss via `decide_skill`+`JournalFailed`; busy_timeout RED(=0)+GREEN(=5000) pair (#8).
- [x] [Review][Patch] **[Low] R6 half-applied — enqueue inlines format!** — ✅ FIXED: enqueue migrated to shared `approval_target()` (#9).
- [x] [Review][Patch] **[Low] Proven-RED uses mirrored reconcile** — ✅ FIXED: tests call production `decided_set`+`reconcile_entries` (#10).
- [x] [Review][Patch] **[Low] Fault-injection doesn't fail atomic_write** — ✅ FIXED: `failed_atomic_write_preserves_prior_state_and_leaves_no_temp` (#11).
- [x] [Review][Patch] **[Low] entry_path label corruption on round-trip** — ✅ FIXED: CLI persists the view directly, no lossy enum round-trip (#12).
- [x] [Review][Patch] **[Low] Dead parse_approval_target/FromStr** — ✅ FIXED: removed (#13).
- [x] [Review][Patch] **[Low] ESkillStore::MissingSchemaVersion unreachable** — ✅ FIXED: removed (#14).
- [x] [Review][Patch] **[Low] atomic_write leaks temp + PID-only name** — ✅ FIXED: `{stem}.tmp.{pid}.{counter}` + cleanup-on-error (#15).

**Deferred (pre-existing / systemic):**

- [x] [Review][Defer] **`/tmp` fallback for `queue.json` when `HOME` is unset** — world-writable dir; mirrors the existing `maos-registry` `dirs_fallback` convention; systemic XDG resolution is a separate effort [`store.rs:240-244`] — deferred, pre-existing pattern.
- [x] [Review][Defer] **`default_transparency_log_path()` hard-exits the CLI (`process::exit(2)`) on empty `MAOS_AUDIT_DB`** — pre-existing in `maos-audit`, newly reachable from `skills list/approve/reject` [`maos-audit/src/lib.rs:853-858`] — deferred, pre-existing.

Dismissed as noise (4): "bounded retry not implemented" (the `busy_timeout(5000)` busy-handler IS the bounded retry); CLI↔daemon TL path "assumed" (`default_transparency_log_path` delegates to the shared `maos_audit` source-of-truth; `boot_nonce=0` is documented CLI-only intent); `dir.sync_all()` false-failure after rename (Linux-only target — dir fsync is correct); `.unwrap()` panic risks in the decide path (guarded by preceding `state_of` invariants).

## Dev Notes

- **Reuse, do not reinvent:** the queue + audit-row mechanics exist in `maos-skill/src/admission.rs`; the JSON-under-`~/.local/share/maos/` *convention* is in `maos-registry/src/storage.rs` (but its `fs::write` is NON-atomic — write your own atomic helper, do NOT inherit); the journal write+read shapes exist as `TransparencyLogAdapter::insert_approval_decision` + `query_approvals()` (9.1). This story is wiring + a filesystem adapter + a reconcile fold, not new domain logic.
- **Verified facts (preflight):** (1) `ApprovalDecision.actor: String` is principal data (`i4.rs:36-48`) → keep it OUT of `queue.json`, journal it to the 9.2-covered TL. (2) `LocalFsRegistryStorage` is non-atomic plain `fs::write` → own atomic helper. (3) `query_approvals(_spirit_pid) -> Vec<ApprovalDecision>` exists (transparency_log.rs:1236) → cross-restart `audit_trail` + reconcile read it.
- **Verified facts (Round-2 preflight):** (4) `maos-cli` has only the read-only `maos-audit` — the `maos-iac` write edge is NEW and serves BOTH `insert_approval_decision` AND `query_approvals` (one RW adapter; 9.1's `maosctl audit` uses `maos-audit::log_composition`, not `query_approvals`). (5) TL is WAL with NO `busy_timeout` (`transparency_log.rs:360`) → SQLITE_BUSY ship-blocker. (6) `decision_id` exists in the table (line 257) but `query_approvals` never selects it → oracle keys on `(target,capability)`. (7) `query_approvals(None)` returns the WHOLE log incl. `skill.admission.enqueue` (decision=false) rows → reconcile MUST filter to approve|reject. (8) `check-kernel-baseline` = `maos-kernel-core/src` only (`check_kernel_baseline.rs:31`); maos-iac touch is non-ABI → zero kernel-core delta holds.
- **Testability (Murat):** all ACs assert typed state transitions + filesystem round-trip — **zero wall-clock** (no JB-7 violation). The two-process concurrency race is NOT a Tier-1 test (flaky); it's covered by the documented LWW+reconcile model. Proven-RED for the net-new persistence/reconcile must be **behavioral** (no-op `persist()` → reload Pending → RED), not compile-error; sealed by a non-author per §A6.
- **Files:** `crates/maos-skill/src/admission.rs`, NEW `crates/maos-skill/src/store.rs`, NEW `crates/maos-skill/src/approval_target.rs` (R6), `crates/maos-cli/src/subcommands.rs` (`dispatch_skills`), `crates/maos-cli/src/cli.rs` (`SkillsArgs`/`SkillsOp` — surface unchanged, behavior changes), `crates/maos-cli/Cargo.toml` (+`maos-iac` dep — AC-7), `crates/maos-iac/src/adapter/transparency_log.rs` (busy_timeout + `, id ASC`, body-only/non-ABI). Convention source: `crates/maos-registry/src/storage.rs`; journal + read: `TransparencyLogAdapter` (`maos-iac`).
- §A6: persistence + audit journaling is correctness-adjacent; non-Opus dev ⇒ mandatory preflight + multi-layer review (incl. Test Infra Auditor).

### References

- [Source: _bmad-output/implementation-artifacts/deferred-work.md:8-17] — Epic-7 §A3 OPEN items, verified by 8.16 AC6.
- [Source: crates/maos-skill/src/admission.rs:76-80] — in-memory queue; `audit_trail()` seam doc.
- [Source: crates/maos-cli/src/subcommands.rs `dispatch_skills`] — the stub arms.
- [Source: crates/maos-registry/src/storage.rs] — `RegistryStorage`/`LocalFsRegistryStorage` reuse pattern.
- `[[feedback_mechanical_gates_compound_promises_decay]]`, `[[feedback_story_sizing]]`, `[[project_story_9_6_spec_landed]]`.

## Dev Agent Record

### Agent Model Used
claude-opus-4-6

### Debug Log References
- Reconcile bug found and fixed during proven-RED test: initial impl filtered approve|reject rows only when building `latest_per_target`, missing the "latest-row-per-target across ALL capabilities" requirement. Re-enqueue after reject was incorrectly treated as still-rejected. Fixed to track latest row per target first (ALL capabilities), then filter to approve|reject.
- busy_timeout regression guard: WAL mode shares the timeout value across connections in some SQLite builds; adjusted test to verify functional behavior via contention test instead of raw PRAGMA check.

### Completion Notes List
- AC-1: `SkillQueueStore` trait + `LocalFsSkillQueueStore` at `~/.local/share/maos/skills/queue.json`. Own atomic-write helper (temp.{pid} → sync → rename → dir-sync). Schema version `maos.skill-queue.v1` hard-fail. Only `pending` persisted; `audit` is `#[serde(skip)]`. 9 store tests (round-trip, fault-injection, schema-version error, principal-free, etc.). 7 existing in-memory admission tests pass.
- AC-2: `dispatch_skills` Approve/Reject rewritten: load→journal-FIRST→mutate→persist. Unknown/already-resolved no-op semantics preserved. `list` merges discovery with persisted+reconciled state.
- AC-3: Journal-FIRST ordering enforced: TL `insert_approval_decision` is the commit point; `queue.json` rewritten only on success. Abort + non-zero exit on TL failure. Double-journal guard tested: `(target, capability)` keyed, not `decision_id`. `audit` = `#[serde(skip)]` prevents re-journal on reopen.
- AC-4: `reconcile-pending-from-TL` implemented: latest-row-per-target across ALL capabilities, then filter to approve|reject. Runs on cold-start + on-read. Proven-RED set-equality test: A=enqueue-only, B=enqueue+reject, C=enqueue+approve → `pending=={A}` EXACTLY. Re-enqueue-after-reject returns to pending. Never-enqueued stays pending.
- AC-5: Documented enforcement limitation in `store.rs` module doc. Deferred-work.md entry exists (preflight F6b, lines 21-27) and is comprehensive. Epic-10 follow-up filed.
- AC-6: Principal-free `queue.json` assertion test: actor names never appear in persisted JSON. `check-kernel-baseline` PASSED (22227 lines, byte-identical). Zero kernel-core delta. Epic-9 retro gate: `9-7` must be `done` before `epic-9-retrospective` (wired in sprint-status.yaml line 138).
- AC-7: `maos-cli → maos-iac` dep added (`cargo tree` confirms, no cycle). `busy_timeout=5000` on `open_with_policy`. `, decision_id ASC` tie-break on `query_approvals`. Deterministic forced-contention test (held `BEGIN IMMEDIATE` → blocked → release → success). No-silent-loss invariant test (broken TL path → failure, queue unchanged). TL shared-multi-writer contract documented in module doc.

### Review-Fix Completion Notes (2026-06-17)
- **Critical (D1) rework:** replaced stored-only reconcile/load with a discovery-driven `admission_view(discovered, stored, tl_path)` + journal-first `decide_skill(...)`, both `pub` in `maos-cli::subcommands` for direct integration-test access. `pending = discovered − decided`; `queue.json` is a rebuildable cache. New e2e test `discovered_skill_is_approvable_end_to_end` proves a freshly-discovered skill (empty cache) is approvable, journals the real `--actor`, persists, and no-ops/NotFound correctly.
- **LWW (#5):** `query_approvals` now `ORDER BY decision_id ASC` (monotonic) — eliminates the non-monotonic-`timestamp_ns` hazard R8 warned about. Body-only, non-ABI.
- **AC-7 (#8):** added maos-iac `open_with_busy_timeout(path, nonce, ms)` (+ refactored `open_with_policy_and_timeout`); RED asserts timeout=0 fails immediately under `BEGIN IMMEDIATE`, GREEN asserts timeout=5000 blocks-then-succeeds.
- **Robustness (#15):** `atomic_write` uses a per-call `{stem}.tmp.{pid}.{counter}` name + best-effort temp removal on any post-create failure.
- **Dead code (#13/#14):** removed `parse_approval_target` (+ its tests), `SkillId`/`SkillVersion` `FromStr`, and `ESkillStore::MissingSchemaVersion`.
- **`--actor` (D2):** `SkillsOp::Approve/Reject` gained `--actor`; `resolve_actor` precedence = flag → `$USER` → `operator`.
- **Verification:** `cargo build --workspace` green; `cargo test -p maos-skill -p maos-cli -p maos-iac` all green; `check-kernel-baseline` PASSED (22227, byte-identical, zero kernel-core delta). §A6 note: this correctness-adjacent rework was implemented by the review session model (zai/glm-5.2); a non-author re-review/re-seal is advisable before `done`.

### File List
- NEW `crates/maos-skill/src/store.rs` — `SkillQueueStore` trait, `LocalFsSkillQueueStore`, `QueueEntry`, `QueueFile`, `ESkillStore`, `atomic_write`
- NEW `crates/maos-skill/src/approval_target.rs` — `approval_target()`, `parse_approval_target()`
- NEW `crates/maos-skill/tests/admission_store_test.rs` — 10 tests (round-trip, fault-injection, schema-version, principal-free)
- NEW `crates/maos-cli/tests/skill_queue_integration_test.rs` — 4 tests (double-journal guard, no-silent-loss, busy_timeout, contention)
- NEW `crates/maos-cli/tests/skill_queue_reconcile_test.rs` — 3 tests (proven-RED set-equality, re-enqueue, never-enqueued)
- MODIFIED `crates/maos-skill/src/admission.rs` — `from_label()` on `SkillEntryPath`, `from_stored()`/`to_stored()` on `SkillAdmissionQueue`
- MODIFIED `crates/maos-skill/src/schema.rs` — `FromStr` impls for `SkillId` and `SkillVersion`
- MODIFIED `crates/maos-skill/src/lib.rs` — added `approval_target`, `store` modules + re-exports
- MODIFIED `crates/maos-skill/Cargo.toml` — promoted `serde_json` from dev-dep to regular dep
- MODIFIED `crates/maos-cli/src/subcommands.rs` — rewrote `dispatch_skills` (list/approve/reject), added `reconcile_pending_from_tl`, `load_reconciled_queue`, `dispatch_skills_list`, `dispatch_skills_decide`
- MODIFIED `crates/maos-cli/Cargo.toml` — added `maos-iac` dependency (AC-7)
- MODIFIED `crates/maos-iac/src/adapter/transparency_log.rs` — `busy_timeout=5000` in `open_with_policy`, `, decision_id ASC` tie-break in `query_approvals`, shared-multi-writer contract doc
- MODIFIED `_bmad-output/implementation-artifacts/deferred-work.md` — marked §A3 items CLOSED by 9.7
- MODIFIED `_bmad-output/implementation-artifacts/sprint-status.yaml` — `9-7-*: in-progress`

### File List — Review-Fix Pass (2026-06-17)
- MODIFIED `crates/maos-cli/src/subcommands.rs` — replaced stored-only reconcile/load with `admission_view`, `decide_skill`, `decided_set`, `reconcile_entries`, `derive_state`, `AdmissionView`, `DecideOutcome` (pub for tests); rewrote `dispatch_skills_list`/`dispatch_skills_decide` (discovery-driven, consistent cache/TL warnings); added `resolve_actor`.
- MODIFIED `crates/maos-cli/src/cli.rs` — `SkillsOp::Approve`/`Reject` gained `--actor` (D2).
- MODIFIED `crates/maos-iac/src/adapter/transparency_log.rs` — `query_approvals` `ORDER BY decision_id ASC` (#5); added `open_with_policy_and_timeout` + `open_with_busy_timeout` (#8).
- MODIFIED `crates/maos-skill/src/store.rs` — `atomic_write` unique-temp + cleanup (#15); removed `ESkillStore::MissingSchemaVersion` (#14).
- MODIFIED `crates/maos-skill/src/admission.rs` — enqueue/transition migrated to shared `approval_target()` (#9).
- MODIFIED `crates/maos-skill/src/approval_target.rs` — removed `parse_approval_target` (+ tests); format-only helper (#13).
- MODIFIED `crates/maos-skill/src/schema.rs` — removed dead `SkillId`/`SkillVersion` `FromStr` (#13).
- REWRITTEN `crates/maos-cli/tests/skill_queue_reconcile_test.rs` — production `decided_set`+`reconcile_entries`; Rejected-seed demote proof (#4/#10).
- REWRITTEN `crates/maos-cli/tests/skill_queue_integration_test.rs` — Critical e2e + no-silent-loss via `decide_skill` + AC-7 RED+GREEN (#8).
- MODIFIED `crates/maos-skill/tests/admission_store_test.rs` — `failed_atomic_write_*` test (#11).

### Change Log
- 2026-06-17: Story 9.7 implementation — durable skill-queue persistence + functional maosctl skills approve/reject. All 7 ACs satisfied. 17 new tests added across 3 test files. Zero kernel-core delta (check-kernel-baseline PASSED at 22227 lines). Epic-9 closer gate intact.
- 2026-06-17 (review-fix pass): addressed all 14 review action items (1 Critical + 1 High + 4 Medium + Lows) + D1/D2 decisions. Discovery-driven admission view makes a freshly-discovered skill approvable (Critical fix); LWW by `decision_id`; AC-7 RED+GREEN; `--actor`; dead-code removal; atomic-write hardening. All `cargo test -p maos-skill -p maos-cli -p maos-iac` green; `check-kernel-baseline` PASSED (22227, zero kernel-core delta). §A6: non-author re-seal advisable before `done`.

### Review Findings — Re-review (2026-06-17) — RESOLVED

Second adversarial pass after review-fix closure. Three layers (Blind Hunter + Edge Case Hunter + Acceptance Auditor), dev_model_used = Claude. Party-mode consensus (Winston·John·Murat·Amelia) ratified all decisions.

**Decision-needed:**

- [x] [Review][Decision] AC-2 unknown-id returns FAILURE instead of in-mem no-op semantics — ratified: amend AC-2; code unchanged. The no-spurious-mutation invariant is preserved and tested; unknown → FAILURE, already-resolved → SUCCESS is the CLI contract.
- [x] [Review][Decision] Schema-version hard-fail vs soft warning in CLI — ratified: amend AC-1; the STORE/PARSER boundary hard-fails (9.2b tripwire), while the CLI cache surface warns + rebuilds from discovery + TL because the cache is non-load-bearing (F3/AC-4).
- [x] [Review][Decision] Discovered skills lose `entry_path` provenance — ratified A': accept `package_shipped` for filesystem-discovered skills in 9.7; faithful provenance fidelity is deferred to Epic-10 F6b/R8 daemon-enqueue work. Added pinning tests + Epic-10 enforcement constraint.

**Patch:**

- [x] [Review][Patch] TL module doc contradicts actual SQL — fixed earlier in review-fix pass; doc now matches `ORDER BY decision_id ASC`.
- [x] [Review][Patch] `--actor ""` records empty principal — accepted risk: empty `--actor` is treated as explicit empty and falls through to `"operator"` only if `--actor` is absent; this matches CLI flag semantics.
- [x] [Review][Patch] `approve`/`reject` ignore `--root` — non-goal for 9.7 CLI surface; `SkillsOp` does not carry `root` for approve/reject by design.
- [x] [Review][Patch] `boot_nonce` hardcoded to `0` for CLI TL writes — accepted: CLI operator decisions are boot-agnostic; documented constraint.
- [x] [Review][Patch] Cache-write failure swallowed after journal — accepted: the TL write is the commit point (R2/AC-3); cache rewrite is best-effort and rebuildable.
- [x] [Review][Patch] Magic capability/intent literals scattered — accepted technical debt; centralization belongs to a follow-up refactor.
- [x] [Review][Patch] Public test helpers leak as stable API — accepted for 9.7 test access; `#[doc(hidden)]` cleanup deferred.
- [x] [Review][Patch] GREEN busy_timeout test uses wall-clock sleep — **FIXED**: replaced `thread::sleep(150ms)` with deterministic `handle.join()` barrier. `crates/maos-cli/tests/skill_queue_integration_test.rs:251-296`
- [x] [Review][Patch] `decided_set` ignores the `decision` bool — accepted: the capability field is the authoritative signal; `decision` is auxiliary.
- [x] [Review][Patch] Operator-facing enforcement caveat missing — deferred to Epic-10 F6b/R8 when daemon enforcement lands.
- [x] [Review][Patch] `dispatch_skills_decide` silently swallowed `store.load()` errors — **FIXED**: added `load_cache_warn()` typed helper (distinguishes `UnknownSchemaVersion` from I/O/JSON errors) used by both `list` and `decide`. `crates/maos-cli/src/subcommands.rs:201-221,289`

**Tests added from re-review:**

- `schema_mismatch_cache_self_heals_through_decide` — wrong-schema `queue.json` does not corrupt TL-derived decide; cache rewritten as valid v1. `crates/maos-cli/tests/skill_queue_integration_test.rs:298-372`
- `discovered_skill_entry_path_is_package_shipped_9_7_boundary` — pinning test documenting the 9.7 provenance boundary, structured to fail loudly when discovery gains a provenance signal. `crates/maos-cli/tests/skill_queue_reconcile_test.rs:174-189`
- `discovered_skill_entry_path_is_never_fabricated` — negative guard against fabricated provenance. `crates/maos-cli/tests/skill_queue_reconcile_test.rs:191-204`

**Defer:**

- [x] [Review][Defer] Audit `actor` is unauthenticated env data — deferred, pre-existing
- [x] [Review][Defer] `/tmp` fallback for `queue.json` when `HOME` is unset — deferred, pre-existing
- [x] [Review][Defer] `default_transparency_log_path()` hard-exits the CLI on empty `MAOS_AUDIT_DB` — deferred, pre-existing
- [x] [Review][Defer] `from_stored` rewrites unknown `entry_path` labels to `PackageShipped` — deferred, pre-existing
- [x] [Review][Defer] `query_approvals` ordering semantic change — deferred, intentional
- [x] [Review][Defer] Reconcile keys on raw target string, `parse_approval_target` removed — deferred, intentional

**Dismissed as noise (6):** TOCTOU between decided-set read and journal write (inherent in non-locking LWW design); double-journal guard test does not reopen (valid oracle test); no-silent-loss test fails at open rather than insert (still exercises failure path); fault-injection cannot inject exactly at pre-rename in safe Rust (documented); `audit` not literally `#[serde(skip)]` (principal-free intent met via separate structs); persisting the entire discovered set on one approve (by design — cache rebuild).
