---
epic: epic-9
epic_title: "Audit & Compliance Surfaces + Operator Productionization (v0.5 → v1.0)"
dev_model_used: claude-opus-4-8  # RECOMMENDED. Persistence + audit-journal drain is correctness-adjacent (durable state, restart survival). §A6: if non-Opus, party-mode preflight + multi-layer review applies.
---

# Story 9.7: Durable Skill-Queue Persistence + Functional `maosctl skills approve/reject`

Status: ready-for-dev

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

**Given** a `SkillQueueStore` port + a `LocalFsSkillQueueStore` writing a **single `~/.local/share/maos/skills/queue.json`** with 9.7's **own atomic-write helper** (temp `queue.json.tmp.{pid}` same-dir → `sync_all()` file → `fs::rename` → `sync_all()` parent dir) — `LocalFsRegistryStorage` is confirmed NON-atomic (plain `fs::write`), so its idiom is the directory/JSON convention to follow but its write path is NOT inherited,
**And** the file carries a top-level `schema_version: "maos.skill-queue.v1"`; an unknown/absent version is a **hard error** (no best-effort parse),
**And** the file persists **ONLY `pending`** (the `Vec<PendingEntry>` state machine — skill-id/version/entry-path/state); the in-mem `audit: Vec<ApprovalDecision>` is `#[serde(skip)]` (it carries `actor`, a principal field — see AC-3/AC-6),
**When** a skill is enqueued, `approve`/`reject` is applied, and a NEW store instance is opened over the same path (= restart),
**Then** the `Admitted`/`Rejected` state + pending set are recovered intact (typed `AdmissionState` assertion, not a string scrape),
**And** a fault-injection test (write-to-temp then fail BEFORE rename) proves the prior valid `queue.json` is left intact + parseable — this test also doubles as the probe confirming the write path is genuinely atomic,
**And** the 7 existing in-memory `admission_queue_test.rs` tests still pass (in-memory remains the unit-layer default; the store is the durable backing).

### AC-2 — Functional `maosctl skills approve/reject`

**Given** `maosctl skills approve <id>` / `maosctl skills reject <id>` (`dispatch_skills`, `crates/maos-cli/src/subcommands.rs`),
**When** the operator runs the command against a pending skill id,
**Then** it loads the durable queue, applies `approve`/`reject` (transitioning the entry), persists atomically, and reports the real outcome — **no more `println!`-only stub**,
**And** approving/rejecting an unknown or already-resolved id returns the existing typed no-op semantics (no spurious mutation), surfaced to the operator,
**And** `maosctl skills list` reflects the persisted state across invocations.

### AC-3 — Journal-on-mutation to the Transparency Log (no double-journal by construction)

**Given** the actor-bearing `ApprovalDecision` (`actor: String`, `maos-domain/src/invariants/i4.rs:36-48`) and the existing `TransparencyLogAdapter::insert_approval_decision` (+ the read-side `query_approvals()` shipped in 9.1),
**When** an `approve`/`reject` succeeds,
**Then** that single decision is journaled to the TL **at the moment of mutation** (one mutation = one row) — NOT drained from a persisted Vec,
**And** because the `audit` Vec is `#[serde(skip)]` (AC-1), reopening the queue cannot re-journal already-recorded decisions — **a double-journal guard test proves it**: approve(x)→persist→reopen→reject(y)→persist, then `query_approvals()` shows exactly one row per decision (`count(x)==1 && count(y)==1 && total==2`), counted by typed `decision_id`,
**And** cross-restart `audit_trail` history is served by `query_approvals()` against the TL (the TL is the audit source of truth), not by the in-mem Vec.

### AC-4 — Reconcile pending-view from the Transparency Log (the recovery story; replaces locking)

**Given** that `queue.json` is a rebuildable cache over the append-only TL (so concurrent `maosctl` writers are documented last-writer-wins on the cache — never on the audit record — and **no file lock is used**),
**When** the pending-view is read (cold-start + on-read),
**Then** a `reconcile-pending-from-TL` step derives the pending set by subtracting the `query_approvals()` decided-set from the discovered-skills set, so a stale `queue.json` entry that the TL shows decided is dropped,
**And** a proven-RED test writes a `queue.json` with a stale `pending` entry for a skill the TL shows decided, runs reconcile, and asserts the pending-view no longer lists it.

### AC-5 — Documented enforcement limitation + filed follow-up (the F6b honesty gate)

**Given** the `maos run` daemon does NOT consult admission state at spirit-load time,
**When** 9.7 ships,
**Then** a prominent documented limitation states that persisted approve/reject is the source of truth for **audit and CLI display**, and that the daemon does **not yet enforce** admission at spirit-load (so a rejected skill is not blocked from loading by this story alone),
**And** a follow-up story (daemon honors admitted-skill state at spirit-load) is **filed in `deferred-work.md` and slotted in Epic 10** — not left as a decaying promise. `done` for 9.7 requires both this doc and the filed story.

### AC-6 — Principal-free `queue.json`, green-at-HEAD, Epic-close gate

**Given** the GDPR cascade (Story 9.2) covers the TL `approval_decision_log`,
**When** the story lands,
**Then** a test/assertion confirms `queue.json` contains **no principal data** (no `actor`/operator id — only skill metadata), so 9.7 introduces no new GDPR-erasure surface (the actor lives only in the 9.2-covered TL),
**And** the full discipline suite is green-at-HEAD with zero disabled gates, kernel-core baseline byte-identical (maos-skill/maos-cli work — **zero kernel-core delta expected**; confirm via `cargo xtask check-kernel-baseline`),
**And** the Epic-9 retro dependency on `9-7 == done` is wired/verified.

## Non-Goals (explicitly OUT of 9.7 — do not "helpfully" add)

- Distributed locking / multi-writer concurrency control / `flock` (LWW + reconcile is the ratified model).
- A generic persistence framework or abstraction over the registry idiom (reuse the convention, don't abstract it).
- Migration tooling (the version stamp is a tripwire; write the migrator when v2 ships).
- Daemon-side admission enforcement (the filed Epic-10 follow-up).
- Refactoring `LocalFsRegistryStorage` to be atomic (separate latent-defect follow-up).

## Tasks / Subtasks

- [ ] **Task 1** (AC-1) — `SkillQueueStore` trait + `LocalFsSkillQueueStore` (single `queue.json`, OWN atomic temp+rename+dir-fsync helper, `schema_version` hard-fail); serde ONLY `pending` (`audit` = `#[serde(skip)]`); restart round-trip + fault-injection tests. Reuse `SkillAdmissionQueue` mechanics.
- [ ] **Task 2** (AC-2) — Rewrite `dispatch_skills` Approve/Reject to load → mutate → persist atomically → report; preserve unknown/already-resolved no-op semantics; `list` reads persisted state.
- [ ] **Task 3** (AC-3) — Journal-on-mutation to the TL via `insert_approval_decision`; `audit_trail` cross-restart reads `query_approvals()`; double-journal guard test (count by typed `decision_id`).
- [ ] **Task 4** (AC-4) — `reconcile-pending-from-TL` (subtract `query_approvals()` decided-set), on cold-start + on-read; proven-RED stale-entry-dropped test.
- [ ] **Task 5** (AC-5) — Write the documented enforcement limitation; file the daemon-enforcement follow-up in `deferred-work.md` + Epic-10.
- [ ] **Task 6** (AC-6) — principal-free `queue.json` assertion; `check-kernel-baseline` (expect byte-identical); green-at-HEAD; wire/verify the retro-blocked-until-9.7-done gate.

## Dev Notes

- **Reuse, do not reinvent:** the queue + audit-row mechanics exist in `maos-skill/src/admission.rs`; the JSON-under-`~/.local/share/maos/` *convention* is in `maos-registry/src/storage.rs` (but its `fs::write` is NON-atomic — write your own atomic helper, do NOT inherit); the journal write+read shapes exist as `TransparencyLogAdapter::insert_approval_decision` + `query_approvals()` (9.1). This story is wiring + a filesystem adapter + a reconcile fold, not new domain logic.
- **Verified facts (preflight):** (1) `ApprovalDecision.actor: String` is principal data (`i4.rs:36-48`) → keep it OUT of `queue.json`, journal it to the 9.2-covered TL. (2) `LocalFsRegistryStorage` is non-atomic plain `fs::write` → own atomic helper. (3) `query_approvals(filter) -> Vec<ApprovalDecision>` already exists (transparency_log.rs:1236) → cross-restart `audit_trail` + reconcile both read it; no new TL read path needed.
- **Testability (Murat):** all ACs assert typed state transitions + filesystem round-trip — **zero wall-clock** (no JB-7 violation). The two-process concurrency race is NOT a Tier-1 test (flaky); it's covered by the documented LWW+reconcile model. Proven-RED for the net-new persistence/reconcile must be **behavioral** (no-op `persist()` → reload Pending → RED), not compile-error; sealed by a non-author per §A6.
- **Files:** `crates/maos-skill/src/admission.rs`, NEW `crates/maos-skill/src/store.rs`, `crates/maos-cli/src/subcommands.rs` (`dispatch_skills`), `crates/maos-cli/src/cli.rs` (`SkillsArgs`/`SkillsOp` — surface unchanged, behavior changes). Convention source: `crates/maos-registry/src/storage.rs`; journal: `crates/maos-iac/src/adapter/transparency_log.rs`.
- §A6: persistence + audit journaling is correctness-adjacent; non-Opus dev ⇒ mandatory preflight + multi-layer review (incl. Test Infra Auditor).

### References

- [Source: _bmad-output/implementation-artifacts/deferred-work.md:8-17] — Epic-7 §A3 OPEN items, verified by 8.16 AC6.
- [Source: crates/maos-skill/src/admission.rs:76-80] — in-memory queue; `audit_trail()` seam doc.
- [Source: crates/maos-cli/src/subcommands.rs `dispatch_skills`] — the stub arms.
- [Source: crates/maos-registry/src/storage.rs] — `RegistryStorage`/`LocalFsRegistryStorage` reuse pattern.
- `[[feedback_mechanical_gates_compound_promises_decay]]`, `[[feedback_story_sizing]]`, `[[project_story_9_6_spec_landed]]`.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

### Change Log
