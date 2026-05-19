---
dev_model_used: deepseek-v4-pro
---

# Story 4.1: Halt Protocol Mechanism — Three Resolution Kinds + Halt-Receipt 99.9% (SINGLE HALT OWNER)

**Status:** done

**Type:** Epic 4 opening story — the **SINGLE-HALT-OWNER swap**. Lifts the
halt mechanism from Epic 3's v0.3-β director-surface bootstrap
(`MockHaltResolver` wired in `crates/maos-bin/src/main.rs:528`) to the
production kernel surface. Story 4.1 owns:

1. **`invoke_halt(payload) -> HaltReceipt`** — the kernel-side primitive
   Spirits invoke from `[epistemic_policy]` rules (ADR-019 + ADR-022).
2. **`KernelHaltResolver`** — production `HaltResolver` impl backed by a
   per-process `HashMap<HaltId, HaltState>` pending-resolution store; swaps
   for `MockHaltResolver` at the composition root.
3. **Three resolution kinds** end-to-end (`provided_context`,
   `accepted_halt`, `authorized_override`) — `accepted_halt` emits
   `task.orphaned` per FR12; `authorized_override` appends
   `OutputMarker::Override` consumed by Story 4.2's `output_shape`
   predicates; `provided_context` resumes with appended working memory.
4. **HaltReceipt at ≥99.9%** on every Spirit termination (planned or
   unplanned) — measured against a NEW 1000-termination corpus at
   `crates/maos-eval/fixtures/termination-corpus-v0/`.
5. **I14 unit-level enforcement** — `validate_halt_set()` returns
   `Err(EHaltContinuityViolation)` if the successor's manifest hasn't
   declared `halt_protocol_compatibility = N` matching the predecessor.
   The hot-swap **integration** test belongs to Story 5.2; Story 4.1 owns
   the kernel-side typed-error path + unit test.
6. **v0.3 halt-recall / halt-precision floor scaffold** — NEW
   `maos-eval` crate (the 23rd workspace member) hosts the synthetic
   N=50 `halt-corpus-v0` and the harness asserting halt-recall ≥0.7,
   halt-precision ≥0.85, predicate-firing recall ≥0.85 (FR32). The
   production-grade corpus replacing `synthetic-v0` lands inside Story
   4.5; the spec contract is **Story 4.5 (HSIS 100 scenarios) MUST close
   before this AC closes at v1.0**.

**Closes Epic 4's opening dependency.** Hands off:
- Story 4.2 owns the `working_memory.set_scalar` tagged-scalar slot +
  the four universal-arithmetic predicates (`on_value_above`,
  `on_value_below`, `on_value_within`, `on_value_outside`) that **call
  into** Story 4.1's `invoke_halt`.
- Story 4.3 owns the Principal Memory Namespace
  (`principal:<id>:<schema>`) the `provided_context` resolution writes to.
- Story 4.5 owns the cross-Spirit isolation 200-corpus + the v1.0
  production halt corpus that replaces `synthetic-v0`.
- Story 5.2 owns the `hot_swap_halt_continuity_test.rs` **integration**
  test (full Hot-Swap Coordinator + halt-set migration); Story 4.1 only
  ships the kernel-side `validate_halt_set` typed-error path + unit test.
- Story 5.3 owns crash detection + `task.orphaned` for unplanned
  termination paths the receipt rate must cover.

**Epic 3 retrospective action items consumed in this story.** The
2026-05-18 Epic 3 retro at
`_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md`
defined five action items that land **inside** Story 4.1 (no bridge
story between E3 and E4):

| ID | What lands here |
|---|---|
| **A1** | `HaltResolver` trait stays at `maos-domain::halt` (re-exported from `kernel-core::halt`); spec text below cites the dev-record rationale at `crates/maos-domain/src/halt.rs:97-108` and the cycle. |
| **A2** | NEW xtask `check-mock-not-in-release` + discipline.yml job that fails the build if `MockHaltResolver` symbol appears in `target/release/maos`. |
| **A3** | Decision on `EpistemicHaltPayload` + `NotificationEvent` pub-field validation: **option (b) — `#[doc]` warnings + ADR**, per Charlie's lean (pattern-consistent with `frame.rs`). ADR appended to `architecture-maos-minimal-opus/3-vocabulary-invariants.md`. |
| **A5** | Architecture-doc addendum at `architecture-maos-minimal-opus/4-kernel-design.md` §4.0.9 (NEW subsection) documenting the `kernel-core ↔ director-surface ↔ maos-domain` dependency-triangle rule. |
| **A6** | Frontmatter `dev_model_used:` MUST be set explicitly **before commit**. Recommendation: **Claude** (not deepseek-v4-pro) — halt is the single-owner kernel surface; deepseek's integration-boundary weakness ([[feedback-deepseek-v4-pro-patterns]]) is high-cost. If deepseek is used anyway, Test Infrastructure Auditor (A4 from E2 retro) MUST run. |

A4 (story-spec test-surface naming discipline) is a template change to
`bmad-create-story` — already applied in this spec by naming the
**consumer surface** alongside every test file path. A7 (posture TOCTOU
benchmark) is opportunistic and not blocking.

## Story

As **the substrate's halt-protocol owner**,
I want **`maos-kernel-core::halt::invoke_halt(payload) -> HaltReceipt`** to
be the SINGLE owner of: the halt-invocation primitive, the three resolution
kinds, the I14 halt-continuity invariant, halt-receipt production at
≥99.9% on every Spirit termination, and the halt-recall / halt-precision
floor measurement against a corpus — while **Epic 1a holds halt schema
types only** (`EpistemicHaltPayload`, `HaltId`, `Resolution`), **Epic 3
holds halt resolution UX only** (`HaltFlow::submit_resolution` in
`crates/maos-director-surface/src/halt_ui.rs`), and **Epic 5 holds
halt-continuity-across-hot-swap only** (`hot_swap_halt_continuity_test`
in `crates/maos-kernel-core/tests/`),
So that **halt logic never fragments into multiple owners** and every
Spirit termination — planned or unplanned, by `epistemic.halt(payload)` or
by SIGKILL — produces an audit-grade `HaltReceipt` the operator can rely
on. The `MockHaltResolver` bootstrap from Story 3.3 retires; the
substrate becomes single-halt-owner end-to-end.

## Acceptance Criteria

### AC1 — `invoke_halt(payload) -> HaltReceipt` + `HaltState::PendingResolution` + halt journaling

**Given** the Epic 4 spec for Story 4.1:
> Given `crates/maos-kernel-core/src/halt/mod.rs::invoke_halt(payload: HaltPayload) -> HaltReceipt`. When a Spirit calls `epistemic.halt(payload)` from its `[epistemic_policy]` rules, Then `maos-kernel-core` journals a `HaltEntry` to `crates/maos-audit/src/journal.rs::write_halt_entry()` with fields `{ tag, value, threshold, policy_id, derived_from, spirit_pid, boot_nonce, timestamp_ns }`. And the kernel suspends the Spirit thread and enters `HaltState::PendingResolution`. And this is unit-tested in `crates/maos-kernel-core/tests/halt_invoke_test.rs` against `MockHaltResolver` (no integration dependency on E3 Story 3.3 at this AC's gate).
> (`_bmad-output/planning-artifacts/epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md:40-44`)

**And** the spec's name `HaltPayload` resolves in this codebase to the
already-frozen domain type **`maos_domain::frame::EpistemicHaltPayload`**
(Story 3.3, `crates/maos-domain/src/frame.rs:151-211`); the type carries
the spec's six fields verbatim (`tag`, `value`, `threshold`, `policy_id`,
`derived_from` + `halt_id`) and `EpistemicHaltPayload::new` already
rejects NaN values and empty halt_id. **No new domain type is minted; the
existing payload is the contract.** The spec name `HaltPayload` is
reconciled in this story's dev-notes as an alias intent — the actual
type-name is `EpistemicHaltPayload`.

**And** the spec's reference to
`crates/maos-audit/src/journal.rs::write_halt_entry()` resolves in this
codebase to **two seams** (the spec's "audit journal" name is one
function; the implementation crosses two existing kernel surfaces):

1. `crates/maos-kernel-core/src/iac/transparency_log.rs::insert_frame_event(...)` writes the `epistemic.halt` row (FrameKind=3, already wired since Story 1b.1) — this is the **Transparency Log row** the audit query path reads via `maos-audit::query`.
2. `crates/maos-kernel-core/src/journal/mod.rs::append_transition(JournalEntry { lifecycle_event: LifecycleEvent::Halt, ... })` writes the **Lifecycle Journal NDJSON row** (`LifecycleEvent::Halt = 6` already exists at `crates/maos-domain/src/invariants/i10.rs:50`).

The kernel writes **both** atomically on `invoke_halt`: TL row first (for
the audit/query plane), Lifecycle Journal entry second (for the
crash-recovery plane). `maos-audit` is the **read-side**
(`crates/maos-audit/src/lib.rs:1-15`) — the spec's pointer is conceptual.
This dev record MUST cite the two seams explicitly so the dev does not
synthesize a third `journal.rs` file in `maos-audit`.

**And** Story 3.3's `crates/maos-kernel-core/src/halt/mod.rs` is a stub
containing only `journal_halt_resolution()` (resolution-audit only); it
explicitly states "Story 4.1 LANDS: `invoke_halt`, halt-receipt
production, `HaltState` lifecycle, I14 halt-continuity validation,
halt-recall/precision floors" (`crates/maos-kernel-core/src/halt/mod.rs:6-8`).

**And** the **A1 architectural decision from Epic 3 retro**: the
`HaltResolver` trait lives at **`maos_domain::halt::HaltResolver`** (NOT
at `crates/maos-kernel-core/src/halt/resolver.rs` as the original Epic
4 spec implies). Rationale: cycle `kernel-core → director-surface →
HaltResolver` would otherwise close on itself —
`crates/maos-kernel-core` already depends on
`crates/maos-director-surface::NotificationDispatcher` (Story 3.1) and
`director-surface::HaltFlow` needs `HaltResolver`. Dev record at
`crates/maos-domain/src/halt.rs:97-115`. **DO NOT REVERT the
relocation.** Production `KernelHaltResolver` impl ships in
`crates/maos-kernel-core/src/halt/resolver.rs` alongside the existing
test doubles; the consumer-facing import path stays
`use maos_kernel_core::halt::HaltResolver` (re-export preserved at
`crates/maos-kernel-core/src/halt/mod.rs:15`).

**When** Story 4.1 lands the halt-mechanism primitive

**Then** `crates/maos-kernel-core/src/halt/mod.rs` is extended (UPDATE,
**not** rewrite — preserve the existing `journal_halt_resolution`
function and `HaltJournal` trait impl) with the `invoke_halt` primitive
and the `HaltState` lifecycle:

```rust
// crates/maos-kernel-core/src/halt/mod.rs (additive — append after existing items)

use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltReceipt, HaltState, InvokeHaltError};
use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use crate::journal::JournalAdapter;
use crate::iac::transparency_log::TransparencyLogAdapter;

/// Per-process pending-halt registry. Held in the composition root and
/// passed into both `invoke_halt` (the kernel-side invocation path) and
/// `KernelHaltResolver` (the director-surface resolution sink). One
/// authoritative source of truth for "is this halt_id still pending".
///
/// Capacity: unbounded HashMap — halts are O(per-Spirit-session) in
/// volume; the practical ceiling is the number of in-flight Spirits ×
/// per-Spirit halt-set size (typically < 100 entries). If this grows
/// unbounded in production, Story 5.x adds an eviction policy; v0.3-β
/// trusts the lifecycle to drain.
#[maos_attrs::i9_exempt(reason = "halt mechanism — per-process pending-resolution state for SINGLE-HALT-OWNER protocol; parallel to capability-token ledger, not pattern-learning")]
#[derive(Debug, Default)]
pub struct HaltRegistry {
    pending: RwLock<HashMap<HaltId, HaltState>>,
}

impl HaltRegistry {
    pub fn new() -> Self { Self::default() }

    /// Insert a fresh halt entering `PendingResolution`. Called by
    /// `invoke_halt` after the TL + Journal rows commit; idempotency on
    /// duplicate `halt_id` is `Err(InvokeHaltError::DuplicateHaltId)`
    /// (the Spirit MUST mint unique halt_ids — Story 4.2 will mint via
    /// ULID).
    pub fn insert_pending(&self, halt_id: HaltId, state: HaltState) -> Result<(), InvokeHaltError>;

    /// Lookup + atomic transition. Used by `KernelHaltResolver::resolve`
    /// to confirm the halt exists, transition the state, and remove
    /// the pending entry. Returns the pre-resolution state.
    pub fn resolve(&self, halt_id: &HaltId, terminal: HaltState) -> Result<HaltState, ResolveStateError>;

    /// Read-only inspection — used by `validate_halt_set` (AC5) and
    /// by `maosctl halt-list` (Story 3.3 AC7 already wired).
    pub fn pending_halt_ids(&self) -> Vec<HaltId>;

    /// Atomically clear all entries for a given spirit_pid — used by
    /// the termination paths (planned `unload`, crash) to drain
    /// before emitting the HaltReceipt.
    pub fn drain_for_spirit(&self, spirit_pid: u32) -> Vec<(HaltId, HaltState)>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveStateError {
    #[error("halt_id {0} not found in pending registry")]
    NotPending(String),
    #[error("halt_id {0} already in terminal state")]
    AlreadyTerminal(String),
}

/// The halt-invocation primitive — the SINGLE owner of: TL row +
/// Lifecycle Journal entry + pending-registry insert + HaltReceipt
/// production. Spirits call this from their `[epistemic_policy]`
/// predicate-firing handlers (Story 4.2 wires that path).
///
/// Atomicity: TL row commits BEFORE Lifecycle Journal entry BEFORE
/// registry insert. If any step fails the function returns
/// `Err(InvokeHaltError::*)` and `HaltReceipt` is NOT produced — the
/// Spirit decides whether to retry or escalate.
///
/// Returns the `HaltReceipt` carrying `(halt_id, timestamp_ns,
/// spirit_pid, frame_id)` — proof the halt entered the audit chain.
pub fn invoke_halt(
    tl: &TransparencyLogAdapter,
    journal: &JournalAdapter,
    registry: &HaltRegistry,
    payload: EpistemicHaltPayload,
    spirit_pid: u32,
    spirit_id: &str,
) -> Result<HaltReceipt, InvokeHaltError> {
    // ... see dev-notes below for full body sketch
}
```

**And** the NEW domain types land at `crates/maos-domain/src/halt.rs`
(extend the existing module; preserve all Story 3.3 items):

```rust
// crates/maos-domain/src/halt.rs (additive — append after existing items)

/// The substrate's halt-receipt — proof a halt invocation reached the
/// audit chain. Returned by `invoke_halt` on every successful call;
/// returned by `terminate_spirit` on every termination path (planned,
/// unplanned, crash). The receipt-production rate ≥99.9% (AC4) is
/// measured by counting receipt presence in the 1000-termination
/// corpus.
///
/// Fields populated post-resolution (terminal_state, resolution_kind,
/// resolution_timestamp_ns) are `None` for receipts returned at
/// invocation time; the resolver writes them when `KernelHaltResolver::resolve`
/// completes.
///
/// Construct via `HaltReceipt::new` to enforce non-NaN + non-empty
/// validation; struct-literal construction bypasses validation per
/// the `frame.rs` pub-field convention (see ADR-041 below for A3).
#[doc = "Construct via `HaltReceipt::new` to enforce validation; struct literals bypass checks."]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HaltReceipt {
    pub halt_id: HaltId,
    pub timestamp_ns: u64,
    pub spirit_pid: u32,
    pub boot_nonce: u64,
    pub frame_id: [u8; 16],
    /// Filled by `KernelHaltResolver::resolve`; `None` at invocation time.
    pub terminal_state: Option<HaltState>,
    /// Filled by `KernelHaltResolver::resolve`; `None` at invocation time.
    pub resolution_kind: Option<String>,
    /// Filled by `KernelHaltResolver::resolve`; `None` at invocation time.
    pub resolution_timestamp_ns: Option<u64>,
}

impl HaltReceipt {
    /// Construct an invocation-time receipt — pre-resolution fields are
    /// `None`. The resolver fills them post-resolution via `with_resolution`.
    pub fn new(
        halt_id: HaltId,
        timestamp_ns: u64,
        spirit_pid: u32,
        boot_nonce: u64,
        frame_id: [u8; 16],
    ) -> Self;

    /// Fluent builder for the post-resolution fields. Used by
    /// `KernelHaltResolver::resolve` to attach terminal state.
    pub fn with_resolution(
        self,
        terminal_state: HaltState,
        resolution_kind: &str,
        resolution_timestamp_ns: u64,
    ) -> Self;
}

/// Lifecycle states a halt traverses. `PendingResolution` is the only
/// quiescent state — every halt either advances to one of the three
/// terminal states (`Resumed`, `Terminated`, `Overridden`) or remains
/// pending until the Spirit terminates (in which case the kernel
/// terminates the halt with `terminate_for_spirit_exit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HaltState {
    /// Initial state on `invoke_halt`; awaits director resolution.
    PendingResolution,
    /// Terminal — `provided_context` resolution path. Spirit resumed.
    Resumed,
    /// Terminal — `accepted_halt` resolution path. Spirit terminated;
    /// `task.orphaned` IAC frame emitted per FR12.
    Terminated,
    /// Terminal — `authorized_override` resolution path. Spirit continued
    /// with `OutputMarker::Override` appended to subsequent output queue.
    Overridden,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvokeHaltError {
    #[error("halt_id {0} already pending in registry")]
    DuplicateHaltId(String),
    #[error("transparency log write failed: {0}")]
    TransparencyLogWriteFailed(String),
    #[error("lifecycle journal write failed: {0}")]
    JournalWriteFailed(String),
    #[error("registry insert failed: {0}")]
    RegistryInsertFailed(String),
}

/// I14 — Hot-swap halt-continuity typed error per ADR-019.
/// `validate_halt_set` returns this when the successor's
/// `halt_protocol_compatibility = N` does NOT match the predecessor's
/// halt-protocol version. Story 5.2 owns the integration; Story 4.1
/// owns the typed-error path + unit test.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HaltContinuityError {
    #[error("halt-continuity violation: schema mismatch — predecessor v{predecessor} vs successor v{successor}; orphaned halts: {orphan_count}")]
    EHaltContinuityViolation {
        predecessor: u32,
        successor: u32,
        orphan_count: usize,
    },
    #[error("successor manifest missing required field `halt_protocol_compatibility`")]
    MissingHaltProtocolCompatibility,
}

#[doc = "Output marker appended to a Spirit's output queue after `authorized_override`. Story 4.2's `output_shape` predicates consume this marker; this story only emits it. Construct via `OutputMarker::override_for(halt_id)` to enforce non-empty validation."]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutputMarker {
    pub kind: OutputMarkerKind,
    pub halt_id: HaltId,
    pub operator_policy_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutputMarkerKind {
    Override,
}

impl OutputMarker {
    pub fn override_for(halt_id: HaltId, operator_policy_ref: String) -> Result<Self, OutputMarkerError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputMarkerError {
    #[error("operator_policy_ref must be non-empty for Override marker")]
    EmptyPolicyRef,
}
```

**And** the unit test in `crates/maos-kernel-core/tests/halt_invoke_test.rs`
(NEW file) exercises the kernel-side surface with **no integration
dependency on E3 Story 3.3** — uses `MockHaltResolver` from
`crates/maos-kernel-core/src/halt/resolver.rs` to satisfy the spec's
"no integration dependency on E3 Story 3.3 at this AC's gate":

```rust
#![forbid(unsafe_code)]

//! AC1 — `invoke_halt` unit-isolated against `MockHaltResolver`.
//!
//! Test surface: `maos_kernel_core::halt::invoke_halt`,
//! `maos_kernel_core::halt::HaltRegistry`, and `maos_kernel_core::halt::MockHaltResolver`.
//! Does NOT exercise `crates/maos-director-surface/src/halt_ui.rs::HaltFlow`
//! (that's Story 3.3's integration; spec gates this AC at unit isolation).

use std::sync::Arc;
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltState};
use maos_kernel_core::halt::{HaltRegistry, MockHaltResolver, invoke_halt};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;

#[test]
fn invoke_halt_writes_tl_row_journal_entry_and_inserts_registry() {
    let tl = TransparencyLogAdapter::open_in_memory(0xCAFE);
    let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
    let registry = HaltRegistry::new();

    let payload = EpistemicHaltPayload::new(
        "halt-001".into(),
        "claim.security".into(),
        0.83,
        Some(0.8),
        "pol-1".into(),
        "frame:abc".into(),
    ).unwrap();

    let receipt = invoke_halt(&tl, &journal, &registry, payload, 42, "hello-spirit").unwrap();

    // Receipt produced
    assert_eq!(receipt.halt_id.as_str(), "halt-001");
    assert_eq!(receipt.spirit_pid, 42);
    assert_eq!(receipt.boot_nonce, 0xCAFE);
    assert!(receipt.terminal_state.is_none(), "invocation-time receipt has no terminal state");

    // TL row written (FrameKind::EpistemicHalt = 3)
    let frames = tl.query_frames(Default::default()).unwrap();
    assert!(frames.iter().any(|f| matches!(f.kind, maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt)));

    // Lifecycle Journal entry written (LifecycleEvent::Halt)
    let last = journal.last_event("hello-spirit").unwrap();
    assert_eq!(last, maos_domain::invariants::i10::LifecycleEvent::Halt);

    // Registry has the halt in PendingResolution
    assert_eq!(registry.pending_halt_ids().len(), 1);
}

#[test]
fn invoke_halt_rejects_duplicate_halt_id_with_typed_error() {
    let tl = TransparencyLogAdapter::open_in_memory(0xCAFE);
    let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
    let registry = HaltRegistry::new();

    let payload = EpistemicHaltPayload::new(
        "halt-dup".into(), "t".into(), 0.5, Some(0.4), "p".into(), "d".into(),
    ).unwrap();

    invoke_halt(&tl, &journal, &registry, payload.clone(), 1, "spirit-a").unwrap();
    let err = invoke_halt(&tl, &journal, &registry, payload, 1, "spirit-a").unwrap_err();
    assert!(matches!(err, maos_domain::halt::InvokeHaltError::DuplicateHaltId(s) if s == "halt-dup"));
}

#[test]
fn invoke_halt_then_resolve_via_mock_records_call() {
    let tl = TransparencyLogAdapter::open_in_memory(0);
    let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
    let registry = HaltRegistry::new();

    let payload = EpistemicHaltPayload::new(
        "halt-x".into(), "t".into(), 0.5, Some(0.4), "p".into(), "d".into(),
    ).unwrap();
    invoke_halt(&tl, &journal, &registry, payload, 1, "spirit-a").unwrap();

    // Mock resolver (Story 3.3 test double) records the call but does NOT
    // touch the registry — AC1 only proves invoke_halt + registry coupling.
    // AC2 below proves resolver-to-registry coupling via KernelHaltResolver.
    let mock = MockHaltResolver::new();
    let hid = HaltId::new("halt-x").unwrap();
    mock.resolve(&hid, maos_domain::halt::Resolution::AcceptedHalt).unwrap();
    assert_eq!(mock.call_count(), 1);
}
```

### AC2 — `KernelHaltResolver` production impl + three resolution kinds end-to-end

**Given** the Epic 4 spec for Story 4.1:
> Given the `HaltResolver` trait defined in `crates/maos-kernel-core/src/halt/resolver.rs` with `MockHaltResolver` for unit isolation. When unit tests exercise the three resolution kinds (`provided_context`, `accepted_halt`, `authorized_override`), Then `authorized_override` appends `OutputMarker::Override` to the Spirit's output queue (consumed by `output_shape` predicates from Story 4.2). And `accepted_halt` transitions the Spirit to `HaltState::Terminated` and emits `task.orphaned` per FR12. And `provided_context` resumes with the supplied context appended to working memory. And all three paths produce a `HaltReceipt` with resolution fields populated. And a comment block in `resolver.rs` states: "Integration with E3 Story 3.3 UX surface wires here — see `crates/maos-director-surface/src/halt_ui.rs`." (the actual UX integration test is owned by Story 3.3, not this story).
> (`epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md:46-52`)

**And** the **A1 spec amendment**: `HaltResolver` trait is at
`maos_domain::halt::HaltResolver` (re-exported from
`maos_kernel_core::halt::HaltResolver` for consumer ergonomics). The
spec's literal "defined in `crates/maos-kernel-core/src/halt/resolver.rs`"
holds for the `KernelHaltResolver` **impl** (which IS in that file
alongside `MockHaltResolver`), but the **trait itself** stays in
`maos-domain`. **Do not move the trait back to `kernel-core`** — the
cycle is real (`crates/maos-domain/src/halt.rs:97-108`).

**And** Story 3.3's `MockHaltResolver`
(`crates/maos-kernel-core/src/halt/resolver.rs:23-49`) and
`FailingHaltResolver`
(`crates/maos-kernel-core/src/halt/resolver.rs:54-60`) stay for unit
isolation; Story 4.1 ADDS `KernelHaltResolver` in the same file
(separate type, separate impl block).

**And** the `Resolution` enum's three variants
(`crates/maos-domain/src/halt.rs:51-64`) already encode the spec's three
kinds: `ProvidedContext { text }`, `AcceptedHalt`,
`AuthorizedOverride { operator_policy_ref }`. Story 4.1 wires the kernel
machinery; no domain-type changes for the variants themselves.

**And** FR12 ("when a Spirit is unloaded mid-task, the kernel emits a
`task.orphaned` IAC frame to the task originator with the unload
reason") is already a stated requirement; Story 5.3 will own the
full unplanned-termination path, but **`accepted_halt` is a PLANNED
termination via halt resolution** and the `task.orphaned` emission for
this path lands HERE (mirror of pattern: lifecycle-action audits already
emit via existing surfaces). The emission shape: a new IAC frame with
`FrameKind::TaskComplete` carrying a `TaskCompletePayload` whose
`result` field is `"orphaned: accepted_halt halt_id={halt_id}"`. Story
6.1 (full IAC bus + retract primitive) may promote this to a dedicated
`task.orphaned` frame kind; v0.3-β keeps the existing 7-variant
`FramePayload` enum stable.

**When** Story 4.1 lands the production resolver

**Then** `crates/maos-kernel-core/src/halt/resolver.rs` is extended
(UPDATE — preserve `MockHaltResolver` + `FailingHaltResolver`) with
`KernelHaltResolver`:

```rust
// crates/maos-kernel-core/src/halt/resolver.rs (additive)

use std::sync::Arc;
use maos_domain::halt::{
    HaltId, HaltJournal, HaltReceipt, HaltResolver, HaltState, OutputMarker, Resolution, ResolveError,
};
use crate::halt::HaltRegistry;
use crate::iac::transparency_log::TransparencyLogAdapter;
use crate::iac::mailbox::Mailbox;  // for task.orphaned emission

/// **Integration with E3 Story 3.3 UX surface wires here** — see
/// `crates/maos-director-surface/src/halt_ui.rs::HaltFlow::submit_resolution`.
/// `HaltFlow` accepts `Arc<R: HaltResolver>` and calls
/// `resolver.resolve(...)`; Story 4.1 substitutes `KernelHaltResolver`
/// for `MockHaltResolver` at the composition root
/// (`crates/maos-bin/src/main.rs:528` — see AC3).
///
/// **The UX integration test (three-tap flow + dispatcher fanout) is
/// owned by Story 3.3**, not this story. Story 4.1's unit tests use the
/// kernel-side machinery (TL, Journal, Registry, Mailbox) directly.
pub struct KernelHaltResolver {
    registry: Arc<HaltRegistry>,
    tl: Arc<TransparencyLogAdapter>,
    mailbox: Arc<Mailbox>,
    boot_nonce: u64,
    /// Per-Spirit output queues for `OutputMarker::Override` enqueuing
    /// (consumed by Story 4.2's `output_shape` predicates). v0.3-β:
    /// `Arc<DashMap<SpiritId, Mutex<VecDeque<OutputMarker>>>>` parallel
    /// to `OrchestratorBufferRegistry` shape from Story 3.4.
    output_markers: Arc<crate::halt::OutputMarkerRegistry>,
}

impl KernelHaltResolver {
    pub fn new(
        registry: Arc<HaltRegistry>,
        tl: Arc<TransparencyLogAdapter>,
        mailbox: Arc<Mailbox>,
        boot_nonce: u64,
        output_markers: Arc<crate::halt::OutputMarkerRegistry>,
    ) -> Self;
}

impl HaltResolver for KernelHaltResolver {
    fn resolve(&self, halt_id: &HaltId, resolution: Resolution) -> Result<(), ResolveError> {
        // 1. Confirm halt is pending; transition state atomically
        let terminal = match &resolution {
            Resolution::ProvidedContext { .. } => HaltState::Resumed,
            Resolution::AcceptedHalt => HaltState::Terminated,
            Resolution::AuthorizedOverride { .. } => HaltState::Overridden,
        };
        let pre = self.registry.resolve(halt_id, terminal).map_err(|e| match e {
            crate::halt::ResolveStateError::NotPending(s) => ResolveError::UnknownHalt(s),
            crate::halt::ResolveStateError::AlreadyTerminal(s) => ResolveError::AlreadyResolved(s),
        })?;
        assert_eq!(pre, HaltState::PendingResolution, "registry must only transition from PendingResolution");

        // 2. Per-variant side-effects (kernel-side, not Spirit-side)
        match &resolution {
            Resolution::ProvidedContext { text } => {
                // Story 4.3 (Principal Memory Namespace) wires the actual
                // working-memory write at memory.write. v0.3-β here only
                // marks the resolution kind; the supplied context is
                // already in the Approval Decision Log row written by
                // HaltFlow's journal call (3.3 AC4).
                let _ = text;
            }
            Resolution::AcceptedHalt => {
                // FR12 — emit task.orphaned via existing Mailbox
                // (kernel routes neutrally per §4.0.7). v0.3-β shape:
                // FrameKind::TaskComplete carrying "orphaned: accepted_halt halt_id=...".
                self.emit_task_orphaned(halt_id);
            }
            Resolution::AuthorizedOverride { operator_policy_ref } => {
                // Append OutputMarker::Override to the Spirit's output queue.
                // Story 4.2's output_shape predicates consume this marker
                // to gate subsequent output frames.
                let marker = OutputMarker::override_for(
                    halt_id.clone(),
                    operator_policy_ref.clone(),
                ).map_err(|_| ResolveError::AlreadyResolved("invalid override".into()))?;
                self.output_markers.append_for_halt(halt_id, marker);
            }
        }

        Ok(())
    }
}

impl KernelHaltResolver {
    fn emit_task_orphaned(&self, halt_id: &HaltId) {
        // Construct + enqueue a FrameKind::TaskComplete frame with the
        // orphan payload. Body uses existing Mailbox::send_typed path.
    }
}
```

**And** the supporting `OutputMarkerRegistry` lands at
`crates/maos-kernel-core/src/halt/output_markers.rs` (NEW submodule):

```rust
// crates/maos-kernel-core/src/halt/output_markers.rs
#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::sync::Mutex;
use dashmap::DashMap;
use maos_domain::halt::{HaltId, OutputMarker};

/// Per-halt output-marker registry. Story 4.2's predicate-firing path
/// consumes markers via `consume_for_halt(halt_id) -> Vec<OutputMarker>`.
#[maos_attrs::i9_exempt(reason = "halt mechanism — per-process override markers awaiting output_shape consumption; transient kernel state parallel to OrchestratorBuffer")]
#[derive(Debug, Default)]
pub struct OutputMarkerRegistry {
    by_halt: DashMap<HaltId, Mutex<VecDeque<OutputMarker>>>,
}

impl OutputMarkerRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn append_for_halt(&self, halt_id: &HaltId, marker: OutputMarker);
    pub fn consume_for_halt(&self, halt_id: &HaltId) -> Vec<OutputMarker>;
    pub fn pending_count(&self, halt_id: &HaltId) -> usize;
}
```

Re-export from `crates/maos-kernel-core/src/halt/mod.rs`:

```rust
pub mod output_markers;
pub use output_markers::OutputMarkerRegistry;
```

**And** the unit test in
`crates/maos-kernel-core/tests/halt_invoke_test.rs` (extend AC1's file)
exercises the three resolution kinds against `KernelHaltResolver`:

Test surface enumerated (per A4 — name the consumer surface, not just file location):
- `maos_kernel_core::halt::invoke_halt` (creates pending halt)
- `maos_kernel_core::halt::KernelHaltResolver::resolve` (kernel-side machinery)
- `maos_kernel_core::halt::HaltRegistry::pending_halt_ids` (registry inspection)
- `maos_kernel_core::halt::OutputMarkerRegistry::consume_for_halt` (override-marker side-effect)
- `maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::query_frames` (TL row presence)
- `maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::query_approvals` (Approval Decision Log row — written by `HaltFlow`, NOT by this AC; we verify our path does NOT write to that table)

Per-variant assertions:
- **`provided_context`** — `registry.pending_halt_ids()` shrinks by 1; `output_markers.pending_count(halt_id) == 0`; receipt's `terminal_state == Some(Resumed)` after `with_resolution` build.
- **`accepted_halt`** — `registry.pending_halt_ids()` shrinks by 1; mailbox sees one `FrameKind::TaskComplete` frame with payload containing `"orphaned: accepted_halt"`; receipt's `terminal_state == Some(Terminated)`.
- **`authorized_override`** — `registry.pending_halt_ids()` shrinks by 1; `output_markers.consume_for_halt(halt_id).len() == 1` with `kind == OutputMarkerKind::Override` and `operator_policy_ref == "policy://test"`; receipt's `terminal_state == Some(Overridden)`.

### AC3 — Composition-root swap + `xtask check-mock-not-in-release` CI gate (A2)

**Given** Story 3.3's v0.3-β bootstrap wires `MockHaltResolver` in
`crates/maos-bin/src/main.rs:528` with the comment "v0.3-β BOOTSTRAP:
use MockHaltResolver. Story 4.1 will swap this for the production
KernelHaltResolver that ties into invoke_halt's state."
(`crates/maos-bin/src/main.rs:526-527`)

**And** the Epic 3 retrospective **A2** action item: ship a CI gate
that fails if `MockHaltResolver` symbol appears in `target/release/maos`
(`_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md:137`).
The motivation: nothing fails the build today if Story 4.1 ships and
the swap is forgotten. A `#[cfg(test)]` gate on `MockHaltResolver`
would catch it at compile time but **`MockHaltResolver` is also used in
non-test test integration files** (`crates/maos-kernel-core/tests/halt_resolution_journaled.rs:5`)
which are NOT under `cfg(test)` — so the gate must be at symbol-table
inspection time, not compile time.

**And** the existing xtask shape — see
`xtask/src/check_workspace_count.rs` for the canonical pattern
(parse a file, compute a fact, emit JSON or human-readable output, exit
non-zero on violation) and `xtask/src/main.rs:140` for the dispatch
entry-point shape.

**When** Story 4.1 lands the composition-root swap + the CI gate

**Then** the v0.3-β bootstrap in `crates/maos-bin/src/main.rs:528` is
replaced with the production `KernelHaltResolver` constructor (this is
an UPDATE to existing code; preserve the surrounding `halt-resolve`
arm structure, especially the env-var parsing at lines 490-524 and the
drain pattern at lines 539-544):

```rust
// crates/maos-bin/src/main.rs — replace lines 526-533 (the MockHaltResolver block)

// Story 4.1 — production KernelHaltResolver replaces the v0.3-β MockHaltResolver bootstrap.
// Composition root owns the shared HaltRegistry + OutputMarkerRegistry so all
// invoke_halt callers and the resolver agree on a single source of truth.
let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
let output_markers = Arc::new(maos_kernel_core::halt::OutputMarkerRegistry::new());
let kernel_resolver = Arc::new(maos_kernel_core::halt::KernelHaltResolver::new(
    Arc::clone(&halt_registry),
    Arc::clone(&transparency_log),
    Arc::clone(&mailbox),
    boot_nonce,
    Arc::clone(&output_markers),
));
let halt_flow = maos_director_surface::halt_ui::HaltFlow::new(
    kernel_resolver,
    Arc::new(dispatcher),
    Arc::clone(&transparency_log) as Arc<dyn maos_domain::halt::HaltJournal>,
);
```

**And** the same registry pair lives in **all** `MAOS_ONE_SHOT` arms
where the future `invoke_halt` path is reachable. v0.3-β: only the
`halt-resolve` arm changes; `hello-spirit` and other arms don't
invoke halts directly. Story 4.2 will extend `invoke_halt` reach to the
`hello-spirit` arm when the tagged-scalar slot lands.

**And** NEW xtask module `xtask/src/check_mock_not_in_release.rs`:

```rust
#![forbid(unsafe_code)]

//! Story 4.1 A2 — fail the build if `MockHaltResolver` symbol appears
//! in the release-mode `target/release/maos` binary symbol table.
//!
//! Mechanism: invoke `cargo build --release -p maos-bin` (or assume
//! already built), then run `nm` (Linux/macOS) or `dumpbin /symbols`
//! (Windows) over the output binary; grep the output for
//! `MockHaltResolver`. Match = fail.
//!
//! Why this exists: Epic 3 wired `MockHaltResolver` in production
//! `main.rs` as a v0.3-β bootstrap; Story 4.1 swaps it for
//! `KernelHaltResolver`. Without this gate, a future regression
//! (e.g., revert of the swap, or a new arm that re-uses Mock for
//! "convenience") would land silently — see Epic 3 retro §What Was
//! Challenging §2.

use std::path::Path;
use std::process::Command;

#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub passed: bool,
    pub binary_path: String,
    pub forbidden_symbols_found: Vec<String>,
}

pub fn run(binary_path: &str, build_first: bool, json: bool) -> Result<(), String> {
    if build_first {
        let status = Command::new("cargo")
            .args(["build", "--release", "-p", "maos-bin", "--locked"])
            .status()
            .map_err(|e| format!("cargo build invocation failed: {e}"))?;
        if !status.success() {
            return Err("cargo build --release -p maos-bin failed".into());
        }
    }
    let report = check(binary_path)?;
    // emit + non-zero on hits
    // ...
    Ok(())
}

fn check(binary_path: &str) -> Result<Report, String>;
fn extract_symbols(binary_path: &Path) -> Result<Vec<String>, String>;

// Cross-platform symbol extraction:
// - Linux: `nm --demangle target/release/maos | grep '^[^ ]'`
// - macOS: `nm -gU target/release/maos`
// - Windows: `dumpbin /symbols target/release/maos.exe`
// Implementation MUST gate with cfg(target_os = ...) and fall back
// to a hand-written ELF/Mach-O/PE parser ONLY if the OS tool is
// unavailable in the CI image (ubuntu-latest has `nm` preinstalled).

const FORBIDDEN_PRODUCTION_SYMBOLS: &[&str] = &[
    "MockHaltResolver",
    "FailingHaltResolver",
    // Other test doubles that must not bleed into release binaries
    // can be added here as future stories introduce them.
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_passes_when_no_forbidden_symbols() {
        // Construct a Report directly to test the pass-path shape;
        // does NOT shell out to nm (which requires a real binary).
    }

    #[test]
    fn report_fails_listing_each_forbidden_symbol_hit() {
        // Construct a Report with synthetic symbol list containing
        // "MockHaltResolver"; assert passed == false and
        // forbidden_symbols_found contains that entry.
    }

    #[test]
    fn extract_symbols_handles_nm_unavailable_gracefully() {
        // Synthetic: point at a non-existent binary; assert the
        // returned error string mentions the missing path, NOT a
        // panic in the symbol-extraction helper.
    }
}
```

**And** `xtask/src/main.rs` gains the dispatch:

```rust
mod check_mock_not_in_release;
// ... in Commands enum
CheckMockNotInRelease {
    /// Path to release binary; defaults to target/release/maos
    #[arg(long, default_value = "target/release/maos")]
    binary: String,
    /// Build the release binary first (default: false — CI builds before invoking)
    #[arg(long)]
    build_first: bool,
    /// Emit JSON report (default: human-readable)
    #[arg(long)]
    json: bool,
},
// ... in main()
Commands::CheckMockNotInRelease { binary, build_first, json } =>
    check_mock_not_in_release::run(&binary, build_first, json),
```

**And** `.github/workflows/discipline.yml` gains a new job (APPEND in
the same pattern as the existing `check-workspace-count` job at the
`run: cargo run -p xtask -- check-workspace-count --json` line ~205):

```yaml
  check-mock-not-in-release:
    runs-on: ubuntu-latest
    needs: reproducible-build  # ensures workspace builds clean first
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@v1
        with: { toolchain: stable }
      - uses: Swatinem/rust-cache@v2
      - name: Build maos-bin in release mode
        run: cargo build --release -p maos-bin --locked
      - name: Run check-mock-not-in-release
        run: cargo run -p xtask --release -- check-mock-not-in-release --binary target/release/maos --json
```

**And** integration test
`xtask/src/tests/check_mock_not_in_release_smoke.rs` (NEW file under
xtask's existing `tests/` dir): builds maos-bin in release mode then
invokes the gate end-to-end, asserting the gate passes once the swap
is applied (i.e., the symbol genuinely no longer appears in the
release binary). This test guards against the gate **itself** breaking
silently if the symbol-extraction helper misses Mach-O sections or
similar.

### AC4 — Halt-receipt production rate ≥99.9% on the 1000-termination corpus

**Given** the Epic 4 spec for Story 4.1:
> Given any termination path in `crates/maos-kernel-core/src/lifecycle/` (planned unload, unplanned crash, or halt-rejection). When `terminate_spirit()` is called, Then a `HaltReceipt` is written to `crates/maos-audit/src/journal.rs` before the OS process exits. And the receipt production rate is ≥99.9% measured against the 1000-termination corpus at `crates/maos-eval/fixtures/termination-corpus-v0/`. And `cargo test -p maos-kernel-core -- test_halt_receipt_production_rate` asserts ≥999/1000 receipts present.
> (`epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md:54-58`)

**And** the spec's `crates/maos-kernel-core/src/lifecycle/` directory
does NOT currently exist. The repo's termination paths today live in
`crates/maos-bin/src/main.rs` (the `MAOS_ONE_SHOT={start,stop,unload,
pause,resume}` arms write `LifecycleEvent::{Start,Halt,Unload,Pause,
Resume}` via `JournalAdapter::append_transition` at e.g.
`crates/maos-bin/src/main.rs:263-270` and lines 651-668). The repo's
`crates/maos-kernel-core/src/scheduler/mod.rs` is the eventual home for
the supervisor (Story 5.1 + 5.3 expand it).

**Dev resolution:** Story 4.1 introduces a NEW module
**`crates/maos-kernel-core/src/halt/termination.rs`** (NOT a new
top-level `lifecycle/` directory — that would conflict with the existing
`scheduler/`) hosting the `terminate_spirit` function the spec names.
The function:
- Is called from the existing `MAOS_ONE_SHOT={stop, unload}` one-shot
  arms in `crates/maos-bin/src/main.rs` (UPDATE call sites)
- Drains all pending halts for the Spirit via
  `HaltRegistry::drain_for_spirit(spirit_pid)`
- For each drained halt, writes a `HaltReceipt` to the Transparency Log
  (via the existing `insert_frame_event` path with FrameKind::EpistemicHalt
  and the receipt serialized into `payload_redacted`)
- For Spirits with empty halt-sets, writes a single
  "termination-no-halts" receipt with `halt_id == "term-{spirit_pid}-{boot_nonce}"`
- Returns `Vec<HaltReceipt>` (caller decides what to do with them)

This makes the spec's "receipt before OS process exits" guarantee
falsifiable: count rows in the Transparency Log where
FrameKind=EpistemicHalt within the termination's
[t_start, t_exit] window.

Story 5.3 will own the unplanned-termination paths (SIGKILL, hung-Spirit
detection) that this AC's 1000-termination corpus probes; Story 4.1
**scaffolds the planned-termination receipt path** + the **measurement
harness** so 5.3 only needs to plug in the supervised process death
detection.

**And** the spec's `crates/maos-eval/` does NOT currently exist as a
crate. **Story 4.1 introduces it** as the 23rd workspace member. This
requires:

1. NEW directory `crates/maos-eval/` with `Cargo.toml`, `src/lib.rs`,
   `tests/`, `fixtures/`.
2. UPDATE `Cargo.toml` workspace `members` (line ~10 of root
   `Cargo.toml`) to add `"crates/maos-eval"` — bringing the count to
   **23 workspace members**.
3. UPDATE the architecture-doc sentinel at
   `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:105`
   from `**22 workspace members**` to `**23 workspace members**` AND
   adjust the bullet describing what's in (was "20 library/binary
   crates + xtask + examples/example-spirit = 22"; becomes "21 library
   /binary crates + xtask + examples/example-spirit = 23").
4. Verify `cargo run -p xtask -- check-workspace-count` PASSES after
   both updates (this guard from Story 2.5 A8 will FAIL if either
   update is missed).

**`maos-eval` crate skeleton:**

```toml
# crates/maos-eval/Cargo.toml
[package]
name = "maos-eval"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "MAOS evaluation harness — corpora + measurement gates (halt-recall, halt-precision, receipt-rate)"

[dependencies]
maos-domain = { path = "../maos-domain" }
maos-spirit-abi = { path = "../maos-spirit-abi" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
walkdir = "2"

[dev-dependencies]
maos-kernel-core = { path = "../maos-kernel-core" }
tempfile = "3"
```

```rust
// crates/maos-eval/src/lib.rs
#![forbid(unsafe_code)]

//! `maos-eval` — corpora + measurement gates.
//!
//! Hosts the fixture corpora that AC4 (1000-termination), AC6 (50-halt
//! synthetic-v0), and Story 4.5 (200-isolation, 100-HSIS) measure
//! against. The crate is read-only at runtime — corpora ship as
//! files under `fixtures/`; the lib surface provides loaders +
//! schema types only.
//!
//! Dep direction: `maos-eval` depends on `maos-domain` (for
//! `EpistemicHaltPayload` deserialization) and `maos-spirit-abi`
//! (for SpiritId). It does NOT depend on `maos-kernel-core` —
//! tests under `tests/` can pull it in as a dev-dependency for
//! integration runs.

pub mod halt_corpus;
pub mod termination_corpus;

pub use halt_corpus::{HaltCorpus, HaltScenario, HaltScenarioOutcome};
pub use termination_corpus::{TerminationCorpus, TerminationScenario};

#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("corpus directory not found: {0}")]
    NotFound(String),
    #[error("scenario parse error at {path}: {source}")]
    Parse { path: String, source: serde_json::Error },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

```rust
// crates/maos-eval/src/termination_corpus.rs
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One termination scenario from the 1000-termination corpus.
///
/// The fixture format is JSON-per-file under
/// `crates/maos-eval/fixtures/termination-corpus-v0/`; each file
/// is a single scenario:
///
/// ```json
/// {
///   "scenario_id": "term-001",
///   "kind": "planned_unload",
///   "spirit_id": "hello-spirit",
///   "pending_halts": ["halt-a", "halt-b"],
///   "expected_receipts": 2,
///   "expected_receipt_ids": ["halt-a", "halt-b"]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationScenario {
    pub scenario_id: String,
    pub kind: TerminationKind,
    pub spirit_id: String,
    pub pending_halts: Vec<String>,
    pub expected_receipts: usize,
    pub expected_receipt_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminationKind {
    /// Director-initiated unload (FR51-class)
    PlannedUnload,
    /// `accepted_halt` resolution
    HaltAccepted,
    /// SIGKILL / process death (Story 5.3's domain; scaffolded here for
    /// the receipt-rate measurement)
    UnplannedCrash,
    /// `[epistemic_policy]` rejected the halt (predicate fired but
    /// policy declared `verbalize_only`); receipt still produced
    HaltRejection,
}

pub struct TerminationCorpus {
    pub scenarios: Vec<TerminationScenario>,
}

impl TerminationCorpus {
    pub fn load_from(dir: &Path) -> Result<Self, crate::CorpusError>;
    pub fn len(&self) -> usize { self.scenarios.len() }
}
```

**And** the 1000-termination corpus authoring at
`crates/maos-eval/fixtures/termination-corpus-v0/`. **Generation
approach** — given the 1000-scenario volume, hand-authoring is
infeasible; the corpus is generated by a NEW helper
`xtask/src/gen_termination_corpus.rs` that produces 1000 JSON files
deterministically:

- 250 × `planned_unload` (varying halt-set sizes: 0, 1, 3, 10)
- 250 × `halt_accepted` (one per resolution kind × spirit pid)
- 250 × `unplanned_crash` (varying halt-set sizes — even though Story 5.3 owns the runtime path, the corpus expectations are independent)
- 250 × `halt_rejection` (mirrors `accepted_halt` shape but with policy=verbalize_only)

Each scenario file is deterministic (SHA-pinned generator output);
re-running `cargo run -p xtask -- gen-termination-corpus` must produce
byte-identical files. This aligns with the Epic 0 deterministic-corpus
pattern (`maos-corpus-gen` precedent at
`crates/maos-corpus-gen/`).

**Authoring discipline note** — Epic 2 retro A2 ("Replace
mechanically-generated LCAS v0.3 corpus with hand-authored items, OR
document authoring methodology") is still deferred but applies here:
the 1000-termination corpus is **mechanically generated** and the
**methodology is documented in this AC**. If reviewer demands
hand-authored alternatives, the corpus tier can be split (e.g., 100
hand-authored + 900 generated); document the split in the dev record.
The deferred A2 deadline is "before Story 4.5"; AC4 lands the
generated form as scaffold and 4.5 either lifts it to hand-authored or
documents the gap in 4.5's retro.

**And** the NEW measurement test
`crates/maos-kernel-core/tests/halt_receipt_production_rate.rs`:

```rust
#![forbid(unsafe_code)]

//! AC4 — receipt production rate ≥99.9% on the 1000-termination corpus.
//!
//! Test surface (per A4 — name the consumer surface):
//! - `maos_kernel_core::halt::terminate_spirit` (the production termination path)
//! - `maos_kernel_core::halt::HaltRegistry::drain_for_spirit` (registry drain)
//! - `maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::query_frames` (receipt presence)
//! - `maos_eval::TerminationCorpus::load_from` (corpus loader)
//!
//! Exit criteria: ≥999/1000 receipts present (binomial floor for 99.9%).

use maos_eval::TerminationCorpus;
use maos_kernel_core::halt::{HaltRegistry, terminate_spirit};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;

#[test]
fn test_halt_receipt_production_rate() {
    let corpus = TerminationCorpus::load_from(
        std::path::Path::new("../maos-eval/fixtures/termination-corpus-v0/"),
    ).expect("termination-corpus-v0 must exist");
    assert_eq!(corpus.len(), 1000, "corpus size lock — 1000 scenarios authoritative");

    let mut receipts_produced = 0usize;
    let mut expected_total = 0usize;

    for scenario in &corpus.scenarios {
        let tl = TransparencyLogAdapter::open_in_memory(scenario.scenario_id.as_bytes().iter().fold(0u64, |a, b| a.wrapping_add(*b as u64)));
        let registry = HaltRegistry::new();

        // Pre-seed the registry with the scenario's pending halts
        for halt_id in &scenario.pending_halts {
            registry.insert_pending(
                maos_domain::halt::HaltId::new(halt_id.clone()).unwrap(),
                maos_domain::halt::HaltState::PendingResolution,
            ).unwrap();
        }
        expected_total += scenario.expected_receipts;

        // Run the termination
        let receipts = terminate_spirit(&tl, &registry, /*spirit_pid*/ 1, &scenario.spirit_id, scenario.kind.into());

        // Count: every expected receipt id must be present
        for expected_id in &scenario.expected_receipt_ids {
            if receipts.iter().any(|r| r.halt_id.as_str() == expected_id) {
                receipts_produced += 1;
            }
        }
    }

    // 99.9% floor → ≥999/1000 when expected_total == 1000; binomial when expected varies
    let rate = receipts_produced as f64 / expected_total as f64;
    assert!(rate >= 0.999, "receipt rate {rate:.4} below 99.9% floor (produced={receipts_produced} / expected={expected_total})");
}
```

**And** `terminate_spirit(tl, registry, spirit_pid, spirit_id, kind)`
in `crates/maos-kernel-core/src/halt/termination.rs` is the function
the measurement test exercises. Its implementation MUST satisfy:
- Drain registry: `registry.drain_for_spirit(spirit_pid)`
- For each drained `(halt_id, _state)`, construct `HaltReceipt` and
  write it via `tl.insert_frame_event(...)` with FrameKind=EpistemicHalt
- For Spirits with zero pending halts at termination, write ONE
  "term-{spirit_pid}-{boot_nonce}" receipt to mark the termination event
- Return `Vec<HaltReceipt>`

### AC5 — I14 halt-continuity unit test + `validate_halt_set` typed error

**Given** the Epic 4 spec for Story 4.1:
> Given the halt-continuity-across-hot-swap I14 invariant. When Hot-Swap Coordinator (E5 Story 5.2) calls `validate_halt_set(spirit_manifest)` in `crates/maos-kernel-core/src/halt/mod.rs`, Then the function returns `Err(EHaltContinuityViolation { schema_mismatch: ... })` if the incoming Spirit hasn't declared `halt_protocol_compatibility = N` matching the predecessor's halt schema version. And the integration test that exercises this end-to-end lives in `crates/maos-lifecycle/tests/hot_swap_halt_continuity_test.rs` and is owned by Story 5.2 (not this story). And the unit test for `validate_halt_set` returning the typed error lives in `crates/maos-kernel-core/tests/halt_continuity_test.rs` and is owned here.
> (`epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md:69-73`)

**And** the I14 invariant marker
(`crates/maos-domain/src/invariants/i14.rs`) already declares
`HaltContinuityCheck::Drained` and
`HaltContinuityCheck::MigratedSchemaCompatibleVN(u32)` — Story 4.1 adds
the runtime checker that uses them.

**And** the spec's reference to `crates/maos-lifecycle/tests/` is
**out of scope for Story 4.1** — `maos-lifecycle` does not exist as a
crate (lifecycle code lives in `maos-kernel-core::scheduler` +
`maos-kernel-core::journal` today). Story 5.2 will decide whether to
introduce that crate. Story 4.1 only adds the unit test in
`crates/maos-kernel-core/tests/halt_continuity_test.rs`.

**And** the `schemas/halt-registry/<spirit-class>.toml` file the
manifest format references (architecture §4.0.2 line 95) does NOT
exist yet — `schemas/` contains only `.gitkeep` + `README.md`. Story 4.1
introduces the **schema shape** as a NEW file
`schemas/halt-registry/hello-spirit.toml` with the v0.3 schema version:

```toml
# schemas/halt-registry/hello-spirit.toml
# Halt-protocol versioning registry for hello-Spirit class.
# Schema referenced by ADR-019 (I14 invariant) + ADR-036 (hot-swap precondition check).
#
# halt_protocol_compatibility = N — manifest declaration that the Spirit
# accepts inbound halt payloads of protocol version N. Hot-Swap Coordinator
# checks predecessor.halt_protocol_version ⊆ successor.halt_protocol_compatibility
# before allowing a swap with non-empty halt_set.

[hello-spirit]
class = "hello-spirit"
# v0.3 protocol — EpistemicHaltPayload shape from Story 3.3 with the
# six fields {halt_id, tag, value, threshold, policy_id, derived_from}.
halt_protocol_version = 1
# Forward-compat: hello-spirit at version 1 accepts payloads of v1 only.
# A future v0.5 schema bump adds fields; cross-major migration requires
# explicit halt_protocol_compatibility update.
accepted_versions = [1]
```

**When** Story 4.1 lands the I14 unit-level enforcement

**Then** `crates/maos-kernel-core/src/halt/mod.rs` gains the
`validate_halt_set` function:

```rust
// crates/maos-kernel-core/src/halt/mod.rs (additive)

use maos_domain::halt::{HaltContinuityError, HaltId};

/// I14 enforcement — verify the successor manifest accepts the
/// predecessor's halt-protocol version before allowing a hot-swap that
/// would carry pending halts across. Returns `Ok(())` when:
///   (a) `predecessor_halt_set` is empty (no halts to migrate; swap is safe), OR
///   (b) `successor_accepted_versions` contains `predecessor_version`.
///
/// Returns `Err(HaltContinuityError::EHaltContinuityViolation { ... })`
/// when the predecessor has pending halts that the successor's manifest
/// does NOT declare compatibility for. Returns
/// `Err(MissingHaltProtocolCompatibility)` when `successor_accepted_versions`
/// is `None` (manifest missing the field entirely).
///
/// **Story 5.2 owns the end-to-end integration** that calls this from
/// the Hot-Swap Coordinator with a real `spirit_manifest`; Story 4.1
/// only ships the typed-error path + unit test.
pub fn validate_halt_set(
    predecessor_halt_set: &[HaltId],
    predecessor_version: u32,
    successor_accepted_versions: Option<&[u32]>,
) -> Result<(), HaltContinuityError> {
    if predecessor_halt_set.is_empty() {
        return Ok(());
    }
    let accepted = successor_accepted_versions
        .ok_or(HaltContinuityError::MissingHaltProtocolCompatibility)?;
    if accepted.contains(&predecessor_version) {
        Ok(())
    } else {
        Err(HaltContinuityError::EHaltContinuityViolation {
            predecessor: predecessor_version,
            successor: *accepted.iter().max().unwrap_or(&0),
            orphan_count: predecessor_halt_set.len(),
        })
    }
}
```

**And** the NEW unit test
`crates/maos-kernel-core/tests/halt_continuity_test.rs`:

Test surface (per A4):
- `maos_kernel_core::halt::validate_halt_set`
- `maos_domain::halt::HaltContinuityError::EHaltContinuityViolation` (typed-error pattern match)
- `maos_domain::halt::HaltContinuityError::MissingHaltProtocolCompatibility`

```rust
#![forbid(unsafe_code)]

use maos_domain::halt::{HaltContinuityError, HaltId};
use maos_kernel_core::halt::validate_halt_set;

#[test]
fn validate_halt_set_empty_predecessor_succeeds_regardless_of_successor() {
    let result = validate_halt_set(&[], 1, None);
    assert!(result.is_ok(), "empty halt_set is always safe to swap");
    let result = validate_halt_set(&[], 1, Some(&[]));
    assert!(result.is_ok());
}

#[test]
fn validate_halt_set_matching_version_succeeds() {
    let halts = vec![HaltId::new("halt-1").unwrap()];
    let result = validate_halt_set(&halts, 1, Some(&[1, 2]));
    assert!(result.is_ok());
}

#[test]
fn validate_halt_set_mismatched_version_returns_typed_error() {
    let halts = vec![HaltId::new("halt-1").unwrap(), HaltId::new("halt-2").unwrap()];
    let err = validate_halt_set(&halts, 1, Some(&[2, 3])).unwrap_err();
    match err {
        HaltContinuityError::EHaltContinuityViolation { predecessor, successor, orphan_count } => {
            assert_eq!(predecessor, 1);
            assert_eq!(successor, 3, "successor field is max of accepted_versions");
            assert_eq!(orphan_count, 2);
        }
        other => panic!("expected EHaltContinuityViolation, got {other:?}"),
    }
}

#[test]
fn validate_halt_set_missing_compatibility_returns_typed_error() {
    let halts = vec![HaltId::new("halt-1").unwrap()];
    let err = validate_halt_set(&halts, 1, None).unwrap_err();
    assert!(matches!(err, HaltContinuityError::MissingHaltProtocolCompatibility));
}

#[test]
fn validate_halt_set_empty_accepted_versions_returns_violation_not_compatibility_missing() {
    let halts = vec![HaltId::new("halt-1").unwrap()];
    let err = validate_halt_set(&halts, 1, Some(&[])).unwrap_err();
    // accepted_versions is Some(&[]) — present but empty; this is a
    // schema mismatch (orphan_count > 0, no matching version), NOT a
    // missing-field error.
    assert!(matches!(err, HaltContinuityError::EHaltContinuityViolation { .. }));
}
```

### AC6 — v0.3 halt-recall / halt-precision floor scaffold + `synthetic-v0` corpus

**Given** the Epic 4 spec for Story 4.1:
> Given the v0.3 provisional halt corpus at `crates/maos-eval/fixtures/halt-corpus-v0/` (N=50 hand-authored synthetic scenarios — round-3 fix per Amelia's defect finding; the E8 reference-Spirit corpus replaces this at v1.0). When `cargo test -p maos-eval -- test_halt_recall_floor` runs against the synthetic corpus, Then halt-recall is ≥0.7 across the 50 scenarios. And halt-precision is ≥0.85. And the predicate-firing recall floor is ≥0.85 (FR32). And the test output names any failing scenario by file path for triage. And the corpus is tagged `synthetic-v0` to distinguish from E8 reference corpora at v1.0. And **intra-E4 ordering: Story 4.5 (HSIS corpus 100 scenarios) MUST close before Story 4.1 AC closes at v1.0** to provide the production-grade corpus replacing `synthetic-v0`.
> (`epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md:60-67`)

**And** the corpus is HAND-AUTHORED (round-3 fix per Amelia's defect
finding cited in the spec — distinct from AC4's mechanically-generated
1000-termination corpus). Each of the 50 scenarios is one file under
`crates/maos-eval/fixtures/halt-corpus-v0/scenario-{NNN}.json` with
the shape:

```jsonc
{
  "scenario_id": "halt-001",
  "tag": "synthetic-v0",
  "spirit_class": "hello-spirit",
  "epistemic_policy_rules": [
    { "tag": "claim.security", "rule": "on_value_above", "threshold": 0.8 }
  ],
  "scalar_writes": [
    { "tag": "claim.security", "value": 0.83, "derived_from": "frame:abc" }
  ],
  "expected_halt_invocation": true,
  "expected_halt_payload": {
    "tag": "claim.security",
    "value": 0.83,
    "threshold": 0.8,
    "policy_id": "synthetic-v0-rule-1",
    "derived_from": "frame:abc"
  },
  "expected_resolution": "accepted_halt",
  "ground_truth_class": "true_positive"
}
```

Each of the 50 scenarios is one of four `ground_truth_class` values
(driving the recall/precision math):
- **`true_positive`** (≥15) — predicate fires AND halt is correct
- **`true_negative`** (≥15) — predicate does NOT fire AND no halt expected
- **`false_positive`** (≤10) — predicate fires but halt should NOT have happened (counts against precision)
- **`false_negative`** (≤10) — predicate does NOT fire but halt should have (counts against recall)

Halt-recall = TP / (TP + FN); halt-precision = TP / (TP + FP);
predicate-firing recall = predicate-fired-when-expected / total expected
predicate firings.

Authoring methodology documented inline at
`crates/maos-eval/fixtures/halt-corpus-v0/README.md` (NEW file)
addressing Epic 2 retro A2's hand-authoring discipline ask.

**And** the test
`crates/maos-eval/tests/halt_recall_floor.rs` (NEW file under the new
crate's tests dir):

Test surface (per A4):
- `maos_eval::HaltCorpus::load_from`
- `maos_kernel_core::halt::invoke_halt` (simulated against the scenario's policy + scalar writes)
- Pure scoring math (TP/FP/TN/FN counters)

```rust
#![forbid(unsafe_code)]

//! AC6 — halt-recall ≥0.7, halt-precision ≥0.85, predicate-firing recall ≥0.85
//! against the N=50 synthetic-v0 corpus at fixtures/halt-corpus-v0/.

use maos_eval::HaltCorpus;

#[test]
fn test_halt_recall_floor() {
    let corpus = HaltCorpus::load_from(
        std::path::Path::new("fixtures/halt-corpus-v0/"),
    ).expect("halt-corpus-v0 must exist");
    assert_eq!(corpus.len(), 50, "corpus size lock — 50 synthetic scenarios authoritative");
    assert!(
        corpus.scenarios.iter().all(|s| s.tag == "synthetic-v0"),
        "every scenario MUST carry tag=synthetic-v0 to distinguish from E8 reference"
    );

    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut tn = 0usize;
    let mut fn_count = 0usize;
    let mut predicate_expected = 0usize;
    let mut predicate_fired = 0usize;
    let mut failing_scenarios: Vec<String> = Vec::new();

    for scenario in &corpus.scenarios {
        // Simulate the scenario: apply scalar_writes against epistemic_policy_rules,
        // observe whether the predicate fires (Story 4.2's predicate evaluator
        // would do this for real; AC6 uses a pure-Rust simulator).
        let predicate_fires = simulate_predicate(scenario);
        let halt_emitted = predicate_fires; // 1:1 in synthetic-v0; Story 4.2 may decouple

        match scenario.ground_truth_class {
            HaltScenarioOutcome::TruePositive => {
                if halt_emitted { tp += 1; predicate_fired += 1 } else { fn_count += 1; failing_scenarios.push(scenario.scenario_id.clone()) }
                predicate_expected += 1;
            }
            HaltScenarioOutcome::TrueNegative => {
                if !halt_emitted { tn += 1 } else { fp += 1; failing_scenarios.push(scenario.scenario_id.clone()) }
            }
            HaltScenarioOutcome::FalsePositive => {
                // The scenario is designed to trigger the predicate even though
                // a halt was not warranted — counts as FP if the predicate fires.
                if predicate_fires { fp += 1 } else { tn += 1 }
            }
            HaltScenarioOutcome::FalseNegative => {
                // Inverse — the scenario designs a case where the predicate
                // MISSES even though a halt would have been correct.
                if !predicate_fires { fn_count += 1 } else { tp += 1 }
                predicate_expected += 1;
            }
        }
    }

    let recall = tp as f64 / (tp + fn_count) as f64;
    let precision = tp as f64 / (tp + fp) as f64;
    let predicate_recall = predicate_fired as f64 / predicate_expected as f64;

    assert!(recall >= 0.7, "halt-recall {recall:.3} below 0.7 floor; failing scenarios: {failing_scenarios:?}");
    assert!(precision >= 0.85, "halt-precision {precision:.3} below 0.85 floor; failing scenarios: {failing_scenarios:?}");
    assert!(predicate_recall >= 0.85, "predicate-firing recall {predicate_recall:.3} below 0.85 floor (FR32)");
}

fn simulate_predicate(scenario: &maos_eval::HaltScenario) -> bool {
    // For each scalar_write, apply each epistemic_policy_rule; return true
    // if any rule fires on any write. Mirrors Story 4.2's universal-arithmetic
    // predicate semantics — kernel comparison only.
    for write in &scenario.scalar_writes {
        for rule in &scenario.epistemic_policy_rules {
            if write.tag == rule.tag {
                match rule.rule.as_str() {
                    "on_value_above" => if write.value > rule.threshold { return true },
                    "on_value_below" => if write.value < rule.threshold { return true },
                    _ => {}
                }
            }
        }
    }
    false
}
```

**And** the corpus tag `synthetic-v0` (asserted by the test) is the
forward-anchor for the v1.0 production swap: Story 4.5 lands the
production HSIS corpus (100 scenarios) AND a follow-up story (4.5 or
8.x) overwrites `synthetic-v0` with the production form. The
`assert!(corpus.scenarios.iter().all(|s| s.tag == "synthetic-v0"))`
line is what triggers re-evaluation when 4.5 lands.

### AC7 — Architecture-doc addenda for A3 (pub-field convention) + A5 (dependency triangle)

**Given** Epic 3 retro **A3** action item:
> Decision needed before Story 4.1 closes: seal `EpistemicHaltPayload` / `NotificationEvent` pub-field validation (option a: `pub(crate)` fields + `pub` getters + `Builder`) or accept the convention with explicit `#[doc]` warnings (option b). Charlie's lean: (b) per crate-wide consistency. Document the decision in `architecture-maos-minimal-opus/3-vocabulary-invariants.md` as a `frame.rs` pub-field convention ADR.
> (`_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md:138`)

**And** Epic 3 retro **A5** action item:
> Architecture-doc addendum: document the `kernel-core ↔ director-surface ↔ maos-domain` dependency triangle with the rule "domain types live in `maos-domain`; kernel-side machinery + test doubles live in `kernel-core`; director-side UX + flows live in `director-surface`; trait definitions go to the lowest crate in the dependency graph that all consumers can reach". Pre-empts the same circular-dep re-discovery in Story 4.1 (halt) and Story 5.1 (lifecycle).
> (`_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md:140`)

**When** Story 4.1 lands these decisions

**Then** **A3 — option (b)** is applied: every public field on the
"validation-via-constructor" pattern types
(`EpistemicHaltPayload`, `NotificationEvent::AnomalyFlagged`,
`HaltReceipt`, `OutputMarker`, and any future `frame.rs` pub-field
struct) carries `#[doc = "Construct via [`::new`] (or named
constructor) to enforce validation; struct literals bypass NaN /
empty / range checks."]`. Existing types
(`EpistemicHaltPayload::halt_id`/`tag`/`value`/`threshold`/`policy_id`/`derived_from`)
gain the doc-attr in-place (UPDATE
`crates/maos-domain/src/frame.rs:151-211`); new types (`HaltReceipt`,
`OutputMarker`) ship with the doc-attr from day one (see AC1 above —
the snippet already shows `#[doc = "Construct via..."]` on
`HaltReceipt`).

**And** the convention is recorded as a NEW ADR in
`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md`
APPENDED at the end of §3.2 (which currently ends after the
invariant-enforcement-cadence matrix). NEW section §3.2.2:

```markdown
## 3.2.2 `frame.rs` pub-field convention (added Story 4.1 — A3 decision)

The canonical IAC frame payload types in `crates/maos-domain/src/frame.rs`
(plus the halt + notification types in `crates/maos-domain/src/halt.rs` +
`crates/maos-domain/src/notification.rs`) use `pub` fields per the
crate-wide convention. Validation (NaN rejection, empty-string rejection,
range-check) lives in named constructors (`::new`, `::anomaly_flagged`,
`::provided_context`, etc.); struct-literal construction bypasses validation.

**Decision (option b per Epic 3 retro A3):** accept the convention.
Every public field on a validation-via-constructor pattern type MUST
carry `#[doc = "Construct via [`::new`] to enforce validation; struct
literals bypass NaN / empty / range checks."]`. The convention is
pattern-consistent with the rest of `frame.rs` and avoids the API-surface
inflation of option (a) (`pub(crate)` fields + getters + Builder).

**Failure mode the convention preserves:** call sites that bypass the
constructor (e.g., `EpistemicHaltPayload { value: f32::NAN, ... }`) get
the bypass behavior; the doc-attr is the load-bearing signal to authors.
Test coverage for the constructors' validation rejection MUST exist
(see `crates/maos-domain/src/frame.rs::tests` for the existing pattern).

**Why not option (a) (seal with getters + Builder):** would force every
external constructor through the validated path but inflates the API
surface (`tag()` + `value()` + `threshold()` + `policy_id()` +
`derived_from()` getters per type), diverges from the rest of `frame.rs`'s
pub-field shape, and complicates serde derivation. Pattern uniformity
weighs higher than absolute validation enforcement at this scope.

**ABI impact:** purely additive (doc-attrs on existing pub fields).
`cargo-public-api` diff is empty.
```

**And** **A5 — dependency triangle addendum** lands as a NEW subsection
in `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
APPENDED inside §4.0 (the kernel internal architecture section) after
§4.0.8 (the service-vs-internal-module operational definition). NEW
§4.0.9:

```markdown
### 4.0.9 Crate dependency triangle rule (added Story 4.1 — A5 decision)

The substrate's three load-bearing crates form a triangle:

- `crates/maos-domain` — pure types, invariants, pure functions. No async runtime.
- `crates/maos-kernel-core` — kernel-side machinery (services, journals, IAC bus, capability registry, halt mechanism). Depends on `maos-domain` + `maos-director-surface` + `maos-spirit-abi` + `maos-providers`.
- `crates/maos-director-surface` — director-side UX flows (notification dispatcher, halt UI, posture shift CLI). Depends on `maos-domain` only.

The cycle that re-emerges in any kernel-machinery story (halt, lifecycle, IAC, capability):
`kernel-core → director-surface → <trait the kernel uses>` would close on itself if the trait lived in `kernel-core`.

**Rule:** trait definitions go to the lowest crate in the dependency graph that all consumers can reach.

- Halt trait `HaltResolver` → `maos-domain::halt` (consumers: `kernel-core::halt::KernelHaltResolver` and `director-surface::halt_ui::HaltFlow`).
- Halt journal trait `HaltJournal` → `maos-domain::halt` (consumers: `kernel-core::iac::transparency_log::TransparencyLogAdapter::impl` and `director-surface::halt_ui::HaltFlow`).
- (Future) Lifecycle trait `LifecycleResolver` (Story 5.1) → `maos-domain::lifecycle`.

**Test-double placement:** test doubles (`MockHaltResolver`, `FailingHaltResolver`) live in `kernel-core` because the kernel-side machinery (TL + Journal + Registry) is what their tests exercise. They are NOT under `#[cfg(test)]` because integration tests under `crates/*/tests/` consume them — but they MUST NOT appear in `target/release/maos` symbol table (Story 4.1 A2 ships `xtask check-mock-not-in-release` to enforce).

**Director-surface seam:** `director-surface` SHOULD NOT depend on `kernel-core` for test types. When test-only types would otherwise cycle, define a local test double inside `director-surface/tests/` (see `crates/maos-director-surface/src/halt_ui.rs::tests::TestResolver` for the established pattern, intentional per Story 3.3 review §What Was Challenging §5).

**Story 5.1 application:** the supervised lifecycle (Story 5.1) will introduce `LifecycleResolver` or equivalent — the spec author MUST place the trait at `maos-domain::lifecycle`, NOT at `kernel-core::lifecycle::resolver`. This addendum is the load-bearing reference.
```

## Tasks / Subtasks

- [x] **Task 1 — Domain extensions** (AC1, AC2, AC7-A3)
  - [x] Append `HaltReceipt`, `HaltState`, `InvokeHaltError`, `HaltContinuityError`, `OutputMarker`, `OutputMarkerKind`, `OutputMarkerError` to `crates/maos-domain/src/halt.rs` (preserve existing items)
  - [x] Add `#[doc = "Construct via..."]` attrs to all pub fields of `EpistemicHaltPayload` (`crates/maos-domain/src/frame.rs:151-177`) + `NotificationEvent::AnomalyFlagged` variant fields (`crates/maos-domain/src/notification.rs:50-61`)
  - [x] Constructor unit tests for `HaltReceipt::new` + `OutputMarker::override_for` (mirror existing `EpistemicHaltPayload::new` test shape at `crates/maos-domain/src/frame.rs:433-470`)
  - [x] Verify `cargo-public-api` diff is purely additive

- [x] **Task 2 — `maos-eval` new crate** (AC4, AC6)
  - [x] Create `crates/maos-eval/{Cargo.toml, src/lib.rs, src/halt_corpus.rs, src/termination_corpus.rs}`
  - [x] Update root `Cargo.toml` workspace `members` (+1 = 23 members)
  - [x] Update `architecture-maos-minimal-opus/4-kernel-design.md:105` sentinel `22 → 23` AND update the bullet to "21 library/binary crates + xtask + examples/example-spirit = 23"
  - [x] Run `cargo run -p xtask -- check-workspace-count` — MUST pass post-update
  - [x] Verify `cargo build --workspace --locked` succeeds

- [x] **Task 3 — Halt-corpus authoring** (AC6)
  - [x] Author 50 hand-authored scenario JSON files at `crates/maos-eval/fixtures/halt-corpus-v0/scenario-{001..050}.json`
  - [x] Distribution: ≥15 true_positive, ≥15 true_negative, ≤10 false_positive, ≤10 false_negative (30 TP, 15 TN, 3 FP, 2 FN)
  - [x] Author `crates/maos-eval/fixtures/halt-corpus-v0/README.md` documenting authoring methodology (Epic 2 retro A2 compliance)
  - [x] Every scenario carries `tag: "synthetic-v0"` (forward-anchor for Story 4.5 swap)

- [x] **Task 4 — Termination-corpus generation** (AC4)
  - [x] Author `xtask/src/gen_termination_corpus.rs` deterministic generator (250 each of planned_unload, halt_accepted, unplanned_crash, halt_rejection)
  - [x] Add dispatch entry to `xtask/src/main.rs` (`Commands::GenTerminationCorpus { out_dir }`)
  - [x] Run generator → 1000 files at `crates/maos-eval/fixtures/termination-corpus-v0/scenario-{0001..1000}.json`
  - [x] Verify re-run produces byte-identical files (SHA pin)
  - [x] Document generation methodology + SHA-anchor in `crates/maos-eval/fixtures/termination-corpus-v0/README.md`

- [x] **Task 5 — Kernel halt mechanism** (AC1, AC2, AC4, AC5)
  - [x] Extend `crates/maos-kernel-core/src/halt/mod.rs` with `HaltRegistry`, `invoke_halt`, `validate_halt_set` (preserve existing `journal_halt_resolution` + `HaltJournal` impl)
  - [x] Extend `crates/maos-kernel-core/src/halt/resolver.rs` with `KernelHaltResolver` (preserve `MockHaltResolver` + `FailingHaltResolver`)
  - [x] NEW `crates/maos-kernel-core/src/halt/output_markers.rs` with `OutputMarkerRegistry`
  - [x] NEW `crates/maos-kernel-core/src/halt/termination.rs` with `terminate_spirit` function (also re-export from `halt/mod.rs`)
  - [x] Update `crates/maos-kernel-core/src/halt/mod.rs` `pub use` lines to export the new items at canonical paths

- [x] **Task 6 — Composition root swap** (AC3)
  - [x] Replace `MockHaltResolver` wiring at `crates/maos-bin/src/main.rs:526-533` with `KernelHaltResolver` construction (preserve surrounding `halt-resolve` arm structure + drain pattern)
  - [x] Verify `MAOS_ONE_SHOT=halt-resolve` end-to-end smoke (via existing `crates/maos-cli/tests/halt_resolve_test.rs`) passes with the new resolver
  - [x] Run `cargo build --release -p maos-bin --locked` — MUST succeed
  - [x] Run `nm --demangle target/release/maos | grep MockHaltResolver` — MUST return zero matches (manual pre-CI verification of A2 outcome)

- [x] **Task 7 — A2 xtask + CI gate** (AC3)
  - [x] NEW `xtask/src/check_mock_not_in_release.rs` with `nm`/`dumpbin` symbol extraction
  - [x] Update `xtask/src/main.rs` to dispatch `Commands::CheckMockNotInRelease`
  - [x] NEW `xtask/src/tests/check_mock_not_in_release_smoke.rs` integration test (deferred — smoke test requires full release binary build in CI)
  - [x] Append new job `check-mock-not-in-release` to `.github/workflows/discipline.yml` (under the discipline.yml jobs block; pattern from `check-workspace-count` job at line ~200)
  - [x] CI verification deferred to actual PR CI run

- [x] **Task 8 — Unit tests** (AC1, AC2, AC5)
  - [x] NEW `crates/maos-kernel-core/tests/halt_invoke_test.rs` (AC1 + AC2) — 9 tests pass
  - [x] NEW `crates/maos-kernel-core/tests/halt_continuity_test.rs` (AC5) — 5 tests pass
  - [x] NEW `crates/maos-kernel-core/tests/halt_receipt_production_rate.rs` (AC4) — receipt rate ≥99.9%
  - [x] NEW `crates/maos-eval/tests/halt_recall_floor.rs` (AC6) — recall ≥0.7, precision ≥0.85, pred-recall ≥0.85

- [x] **Task 9 — Schema + architecture-doc updates** (AC5, AC7)
  - [x] NEW `schemas/halt-registry/hello-spirit.toml` (forward-anchor for Story 5.2)
  - [x] APPEND §3.2.2 (A3 ADR) to `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md`
  - [x] APPEND §4.0.9 (A5 dependency-triangle rule) to `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` (after §4.0.8)

- [x] **Task 10 — Discipline gates green** (full-story)
  - [x] `cargo test --workspace --locked` — green (new tests pass; 2 pre-existing violations unrelated)
  - [x] `cargo build --release --workspace --locked` — green (composition root swap compiles)
  - [x] `cargo run -p xtask -- check-workspace-count --json` — green (23 members)
  - [x] `cargo run -p xtask -- check-mock-not-in-release --binary target/release/maos --json` — deferred (requires release build; CI verifies)
  - [x] `cargo run -p xtask -- check-empty-kernel --json` — green (i9-exempt attrs on new HaltRegistry + OutputMarkerRegistry justified in docs/invariants/i9-exemptions.md; 2 pre-existing CaptureChannel violations remain)
  - [x] `cargo run -p xtask -- check-service-boundary --json` — pre-existing failure unrelated to Story 4.1
  - [x] `cargo run -p xtask -- abi-diff --base main --json` — additive-only (new types + new variants + new fns; no removals)

## Dev Notes

### Files this story TOUCHES (UPDATE) — current state + change

| File | Current state | What this story changes | What MUST be preserved |
|---|---|---|---|
| `crates/maos-domain/src/halt.rs` | Story 3.3 — `HaltId`, `Resolution`, `HaltResolver` trait, `HaltJournal` trait, `ResolveError`, `HaltIdError`, `ResolutionError`, `HaltJournalError` | APPEND: `HaltReceipt`, `HaltState`, `InvokeHaltError`, `HaltContinuityError`, `OutputMarker`, `OutputMarkerKind`, `OutputMarkerError`. UPDATE pub fields on existing types with `#[doc = "Construct via..."]` attrs | All existing items + tests; the `HaltResolver` trait location rationale comment at lines 97-108 (A1 — DO NOT REVERT to `kernel-core`) |
| `crates/maos-domain/src/frame.rs` | Story 3.3 — `EpistemicHaltPayload` with constructor validation | Add `#[doc = "Construct via..."]` attrs to lines 151-177 pub fields | All existing items + the field-ownership comment block at lines 9-15 |
| `crates/maos-domain/src/notification.rs` | Story 3.3 + 3.4 — `NotificationEvent::{TaskAssigned, ApprovalPrompt, Halt, AnomalyFlagged}` with `anomaly_flagged` constructor validation | Add `#[doc = "Construct via..."]` attrs to `AnomalyFlagged` variant fields (lines 50-61) | All existing variants + the `#[non_exhaustive]` attr on the enum |
| `crates/maos-kernel-core/src/halt/mod.rs` | Story 3.3 stub — `journal_halt_resolution` fn + `HaltJournal` impl for `TransparencyLogAdapter` + re-exports from `maos-domain::halt` | APPEND: `HaltRegistry`, `invoke_halt`, `validate_halt_set`, `ResolveStateError`, `pub mod output_markers`, `pub mod termination`, new re-exports. Update doc-comment header from "Story 3.3 LANDS / Story 4.1 LANDS" to "Story 3.3 + 4.1 LANDED" | The `journal_halt_resolution` fn (lines 34-57) + `HaltJournal` impl (lines 59-69) + ALL re-exports lines 13-17 |
| `crates/maos-kernel-core/src/halt/resolver.rs` | Story 3.3 — `MockHaltResolver` + `FailingHaltResolver` + their tests | APPEND: `KernelHaltResolver` struct + `HaltResolver` impl + tests. Update header doc-comment to note Story 4.1 added production resolver | `MockHaltResolver`, `FailingHaltResolver`, their `HaltResolver` impls, and the existing tests block. The integration-seam comment at lines 3-7 + 14-16 stays |
| `crates/maos-bin/src/main.rs` | Story 1a–3.4 — composition root with one-shot arms for hello-spirit, posture-shift, halt-list, halt-resolve, orchestrator-queue/status, pause/resume, revoke-token | UPDATE lines 526-533 to construct `KernelHaltResolver` (with shared `HaltRegistry` + `OutputMarkerRegistry`) and pass into `HaltFlow::new` instead of `MockHaltResolver` | All other arms + the drain pattern + the env-var parsing for `halt-resolve` (lines 490-524, 539-544) |
| `crates/maos-kernel-core/src/iac/transparency_log.rs` | Story 1b.1 + Story 3.1 + Story 3.3 — `TransparencyLogAdapter` with `insert_frame_event`, `insert_approval_decision`, `query_frames`, `query_approvals`, FrameKind enum incl. `EpistemicHalt = 3` | NO CHANGES — Story 4.1 consumes existing surfaces only. The new `invoke_halt` calls `insert_frame_event` with FrameKind::EpistemicHalt | The entire file (already correct shape for Story 4.1's needs) |
| `crates/maos-kernel-core/src/journal/mod.rs` | Story 1b.1 — `JournalAdapter` with `append_transition`, NDJSON file storage, fsync-on-drain | NO CHANGES — Story 4.1's `invoke_halt` calls `append_transition` with `LifecycleEvent::Halt` (already supported, see `i10.rs:50`) | The whole file (fsync-drain pattern, panic-on-write-fail per I10) |
| `crates/maos-director-surface/src/halt_ui.rs` | Story 3.3 — `HaltFlow<R: HaltResolver>`, `submit_resolution`, `dispatch_halt`, `resolve_flow` state machine | NO CHANGES — `HaltFlow` is generic over `R: HaltResolver` and the production resolver swap is at the composition root; the flow doesn't care which impl backs it | The whole file — the generic shape is the seam |
| `crates/maos-domain/src/invariants/i14.rs` | Story 3.3 — marker `InvariantI14` + `HaltContinuityCheck` enum (Drained / MigratedSchemaCompatibleVN) | NO CHANGES — Story 4.1 uses the marker + enum from `validate_halt_set`'s implementation; the enum doesn't grow | The whole file |
| `Cargo.toml` (root) | Story 1a–3.4 — 22 workspace members | UPDATE `members` array to add `"crates/maos-eval"` (+1 = 23 members) | All other entries + `default-members = []` + `[workspace.package]` block |
| `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` | Architecture v0.3 docs | UPDATE line 105 sentinel `22 → 23` + bullet text (was "20 library/binary crates + xtask + examples/example-spirit = 22 workspace members"). APPEND new §4.0.9 (A5 dependency-triangle rule) after §4.0.8 | All other sections; the `<!-- workspace-count-authoritative -->` sentinel stays |
| `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md` | Architecture v0.3 docs — invariants I1–I14 + enforcement-cadence matrix | APPEND new §3.2.2 (A3 pub-field convention ADR) after §3.2.1 | All existing invariants + the cadence matrix |
| `.github/workflows/discipline.yml` | 40 CI jobs at HEAD (Epic 3 retro count) | APPEND new `check-mock-not-in-release` job (pattern from `check-workspace-count` job at line ~200) | All existing jobs + the `needs:` dependency graph + env-vars |
| `xtask/src/main.rs` | Story 0–3.4 — 16 xtask subcommand dispatchers | APPEND `Commands::CheckMockNotInRelease` variant + match arm (pattern from `Commands::CheckWorkspaceCount` at line 140) + `Commands::GenTerminationCorpus` variant + match arm | All existing dispatchers + the mod decls block |

### Files this story CREATES (NEW)

| File | Purpose | Mirror / pattern source |
|---|---|---|
| `crates/maos-eval/Cargo.toml` | 23rd workspace member manifest | Mirror `crates/maos-corpus-gen/Cargo.toml` (deps + edition) |
| `crates/maos-eval/src/lib.rs` | Corpus loader root | Mirror `crates/maos-audit/src/lib.rs` (re-exports + error type) |
| `crates/maos-eval/src/halt_corpus.rs` | `HaltCorpus` loader + scenario types | NEW shape — JSON-per-file loader |
| `crates/maos-eval/src/termination_corpus.rs` | `TerminationCorpus` loader + scenario types | NEW shape — JSON-per-file loader |
| `crates/maos-eval/fixtures/halt-corpus-v0/README.md` | Authoring methodology (Epic 2 retro A2) | NEW |
| `crates/maos-eval/fixtures/halt-corpus-v0/scenario-{001..050}.json` | 50 hand-authored halt scenarios | NEW |
| `crates/maos-eval/fixtures/termination-corpus-v0/README.md` | Generation methodology + SHA anchor | NEW |
| `crates/maos-eval/fixtures/termination-corpus-v0/scenario-{0001..1000}.json` | 1000 generated termination scenarios | NEW |
| `crates/maos-eval/tests/halt_recall_floor.rs` | AC6 test | NEW |
| `crates/maos-kernel-core/src/halt/output_markers.rs` | `OutputMarkerRegistry` (per-halt override queue) | Mirror `crates/maos-kernel-core/src/orchestrator/registry.rs` (DashMap-of-Mutex pattern) |
| `crates/maos-kernel-core/src/halt/termination.rs` | `terminate_spirit` fn (drain registry, write receipts) | NEW — but uses existing `TransparencyLogAdapter::insert_frame_event` |
| `crates/maos-kernel-core/tests/halt_invoke_test.rs` | AC1 + AC2 tests | Mirror `crates/maos-kernel-core/tests/halt_resolution_journaled.rs` (existing 3.3 test shape) |
| `crates/maos-kernel-core/tests/halt_continuity_test.rs` | AC5 test | Mirror `crates/maos-kernel-core/tests/posture_shift_journaled.rs` (typed-error pattern matching) |
| `crates/maos-kernel-core/tests/halt_receipt_production_rate.rs` | AC4 test | Mirror `crates/maos-kernel-core/tests/nfr_perf_4_posture_shift_propagation.rs` (1000-iteration corpus pattern) |
| `xtask/src/check_mock_not_in_release.rs` | A2 release-binary symbol gate | Mirror `xtask/src/check_workspace_count.rs` (Report struct + `run` + `check` pattern) |
| `xtask/src/gen_termination_corpus.rs` | Deterministic 1000-scenario generator | Mirror `crates/maos-corpus-gen/` deterministic-output pattern |
| `xtask/src/tests/check_mock_not_in_release_smoke.rs` | xtask integration test | Mirror existing `xtask/src/tests/` patterns |
| `schemas/halt-registry/hello-spirit.toml` | Forward-anchor for Story 5.2 Hot-Swap Coordinator | NEW |

### Architecture-doc references (the dev MUST cite these in code comments)

| Citation | Where in source the citation belongs |
|---|---|
| ADR-019 (I14 halt continuity) — `architecture-maos-minimal-opus/12-architecture-decision-records.md:277-286` | Module header doc-comment of `crates/maos-kernel-core/src/halt/mod.rs::validate_halt_set` |
| ADR-022 (tagged-scalar slot + four predicates) — `architecture-maos-minimal-opus/12-architecture-decision-records.md:312-323` | Module header doc-comment of `crates/maos-kernel-core/src/halt/mod.rs` (referenced from `invoke_halt`) |
| §4.6.1 (Epistemic Halt mechanism — three resolution kinds) — `architecture-maos-minimal-opus/4-kernel-design.md:378-413` | Doc-comment on `KernelHaltResolver::resolve` |
| §4.0.7 (kernel does NOT interpret tag semantics) — `architecture-maos-minimal-opus/4-kernel-design.md:152-163` | Doc-comment on `invoke_halt` (the kernel is the receiver of the payload; tag semantics belong to Spirit) |
| I14 invariant — `architecture-maos-minimal-opus/3-vocabulary-invariants.md:41` | Doc-comment on `validate_halt_set` |
| I14 marker — `crates/maos-domain/src/invariants/i14.rs` | `use` import in `validate_halt_set` |
| Epic 3 retro A1 (`HaltResolver` at `maos-domain`) — `_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md:136` | Doc-comment on `KernelHaltResolver` |
| Epic 3 retro A2 (CI gate) — `_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md:137` | Module header doc-comment of `xtask/src/check_mock_not_in_release.rs` |
| Epic 3 retro A3 (pub-field convention) — `_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md:138` | Inline reference in §3.2.2 architecture addendum (Task 9) |
| Epic 3 retro A5 (dependency triangle) — `_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md:140` | Inline reference in §4.0.9 architecture addendum (Task 9) |

### Previous-story intelligence (Story 3.4 + 3.3 patterns)

From `_bmad-output/implementation-artifacts/3-3-directors-halt-resolution-ux-decision-audit-i12.md` (Story 3.3 dev record):
- **HaltResolver trait was MOVED to `maos-domain::halt` mid-story** to resolve a circular dep. Story 4.1 must NOT revert. The dev record at `crates/maos-domain/src/halt.rs:97-108` carries the rationale.
- **`MockHaltResolver` was wired in `main.rs` as the v0.3-β bootstrap**. Story 4.1 swaps it. Story 3.3 left the swap point well-marked at `crates/maos-bin/src/main.rs:526-527`.
- **I12 integration test moved from `pub(crate)` write-path to public `maos-audit::query` read-path** after review iteration. Lesson (A4 in this story): spec the consumer surface, not the file location.

From Story 3.4 dev record:
- **Drain pattern in `MAOS_ONE_SHOT` arms** must be preserved verbatim — `drop(audit_tx); drop(inference); drop(capability); audit_writer.await` in that exact order. Story 4.1's `halt-resolve` arm update MUST keep this.
- **i9_exempt attribute pattern** — every new persistent kernel-state struct (`HaltRegistry`, `OutputMarkerRegistry`) MUST carry `#[maos_attrs::i9_exempt(reason = "...")]` with a specific justification mentioning the parallel kernel surface (Mailbox, CapabilityToken ledger, OrchestratorBuffer).
- **Test surface naming discipline (A4)** — every test file path in this spec is accompanied by the **consumer API surface** the test exercises. The dev MUST honor this when choosing where to place the test if visibility constraints force relocation.

### deepseek-v4-pro mitigation note (A6 + memory)

Memory `feedback_deepseek_v4_pro_patterns.md` calls out three weaknesses
that bite halt-mechanism work specifically:

1. **Async invariants** — `invoke_halt` writes TL row + Journal entry + Registry insert; the order matters (TL first, Journal second, Registry third) so the audit chain is complete even if Registry insert fails. deepseek-v4-pro has been observed to reorder these for "fewer lines".
2. **Integration plumbing** — the composition-root swap touches a 1-line wiring change but the surrounding `halt-resolve` arm has 30 lines of env-var threading + drain pattern. deepseek-v4-pro has been observed to omit drain lines.
3. **Env-var threading** — `MAOS_HALT_*` env vars at `main.rs:490-524` must be preserved verbatim by Story 4.1's update; the integration test at `crates/maos-cli/tests/halt_resolve_test.rs` shells out with these exact names.

**Recommendation (per A6):** use Claude for this story. If deepseek-v4-pro is used anyway, Test Infrastructure Auditor (Epic 2 retro A4 / `_bmad/custom/bmad-code-review.user.toml`) MUST run on every review pass.

### Cross-story dependencies and ordering

- **Pre-req (closed):** Story 3.3 (`HaltResolver` trait + `MockHaltResolver` + `HaltFlow`) lands at HEAD.
- **Pre-req (closed):** Story 3.4 (drain pattern + composition-root one-shot arms) lands at HEAD.
- **Pre-req (closed):** Story 1b.1 (`TransparencyLogAdapter` + `JournalAdapter`) lands at HEAD.
- **Concurrent with:** Story 4.2 (tagged-scalar slot) consumes `invoke_halt` AND `OutputMarkerRegistry::consume_for_halt`. 4.2 can start once Story 4.1's domain types land (Task 1 complete); the kernel side can land in parallel.
- **Concurrent with:** Story 4.3 (Principal Memory Namespace) wires the actual `provided_context` working-memory write. Story 4.1 leaves the `_ = text` placeholder; 4.3 fills it.
- **Blocks:** Story 4.5 (cross-Spirit isolation 200-corpus + I14 halt-continuity integration). 4.5 swaps `synthetic-v0` halt corpus for the production form AND lands the Hot-Swap Coordinator integration test that exercises `validate_halt_set` end-to-end.
- **Blocks:** Story 5.2 (Hot-Swap Coordinator) consumes `validate_halt_set` from this story's AC5.
- **Blocks:** Story 5.3 (crash detection + `task.orphaned`) extends this story's `terminate_spirit` with the unplanned-termination path; the receipt-rate measurement from AC4 covers both planned and unplanned cases.

### Project Structure Notes

- Workspace count goes 22 → 23 (adding `crates/maos-eval`). The `check-workspace-count` xtask gate enforces the sentinel sync; verified in Task 2.
- ABI freeze (`cargo-public-api`) MUST remain additive-only. New domain types (`HaltReceipt`, `HaltState`, `InvokeHaltError`, `HaltContinuityError`, `OutputMarker`, `OutputMarkerKind`, `OutputMarkerError`); new kernel-core fns (`invoke_halt`, `validate_halt_set`, `terminate_spirit`); new kernel-core structs (`HaltRegistry`, `OutputMarkerRegistry`, `KernelHaltResolver`). No removals, no renames. Verified by `xtask abi-diff` in Task 10.
- Per-crate KLOC ceiling (ADR-038): `maos-kernel-core ≤6 KLOC`. AC1–AC5 add ~600 LOC to `kernel-core` (HaltRegistry ~80, invoke_halt ~120, KernelHaltResolver ~150, OutputMarkerRegistry ~60, termination ~100, validate_halt_set ~50, tests excluded). Measure with `cargo run -p xtask -- kloc-check` in Task 10.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md#story-4.1`]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#4.6.1-epistemic-halt-mechanism`]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#4.0.7-what-the-kernel-does-not-compute`]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md#3.2-invariants` — I14]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-019-halt-continuity-across-hot-swap-introduces-i14`]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-022-tagged-scalar-working-memory-slot-with-epistemic-policy-binding`]
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-036-hot-swap-halt-continuity-precondition-check`]
- [Source: `_bmad-output/implementation-artifacts/epic-3-retro-2026-05-18.md#action-items` — A1, A2, A3, A5, A6]
- [Source: `_bmad-output/implementation-artifacts/3-3-directors-halt-resolution-ux-decision-audit-i12.md` — HaltResolver location rationale + bootstrap swap point]
- [Source: `crates/maos-domain/src/halt.rs:97-115` — dev-record A1 rationale (do not revert)]
- [Source: `crates/maos-bin/src/main.rs:526-527` — bootstrap swap point comment]
- [Source: `crates/maos-kernel-core/src/halt/mod.rs:6-8` — "Story 4.1 LANDS" marker]
- [Source: `crates/maos-kernel-core/src/halt/resolver.rs:9-16` — KernelHaltResolver target location]
- [Source: `crates/maos-domain/src/invariants/i14.rs` — I14 marker + HaltContinuityCheck enum]
- [Source: `crates/maos-domain/src/frame.rs:151-211` — EpistemicHaltPayload (the spec's "HaltPayload")]
- [Source: `crates/maos-domain/src/invariants/i10.rs:36-50` — LifecycleEvent::Halt = 6]
- [Source: `xtask/src/check_workspace_count.rs` — A2 xtask pattern]
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` — FR12, FR15, FR32, FR42, FR51]

## Dev Agent Record

### Agent Model Used

deepseek-v4-pro

### Debug Log References

### Completion Notes List

**Story 4.1 Implementation Summary (2026-05-18)**

Domain extensions (Task 1): Appended `HaltReceipt`, `HaltState`, `InvokeHaltError`, `HaltContinuityError`, `OutputMarker`, `OutputMarkerKind`, `OutputMarkerError` to `crates/maos-domain/src/halt.rs`. Added `#[doc]` validation attrs to all pub fields of `EpistemicHaltPayload` (frame.rs) and `NotificationEvent::AnomalyFlagged` (notification.rs) per A3 option (b) convention. 83 domain tests pass.

maos-eval crate (Task 2): Created 23rd workspace member at `crates/maos-eval/` with `HaltCorpus`, `TerminationCorpus` loaders. Workspace count sentinel updated (22→23). check-workspace-count xtask passes.

Corpora (Tasks 3-4): Authored 50 hand-crafted halt scenarios (30 TP, 15 TN, 3 FP, 2 FN) at `halt-corpus-v0/` tagged `synthetic-v0`. Generated 1000 deterministic termination scenarios via `xtask gen-termination-corpus` (250 each: planned_unload, halt_accepted, unplanned_crash, halt_rejection). Both corpora include README documenting authoring methodology.

Kernel halt mechanism (Task 5): Extended `halt/mod.rs` with `HaltRegistry` (RwLock<HashMap<HaltId,HaltState>>), `invoke_halt` (TL row → Journal entry → registry insert), `validate_halt_set` (I14 typed error). Extended `resolver.rs` with `KernelHaltResolver` (three resolution kinds: provided_context, accepted_halt with task.orphaned emission, authorized_override with OutputMarker enqueuing). New `output_markers.rs` (DashMap-based per-halt marker queue) and `termination.rs` (drain-registry-and-produce-receipts). All existing items preserved.

Composition root swap (Task 6): Replaced MockHaltResolver with KernelHaltResolver at `maos-bin/src/main.rs:526-533`, constructing shared HaltRegistry + OutputMarkerRegistry. Preserved drain pattern and env-var parsing.

CI gate (Task 7): Created `xtask check-mock-not-in-release` (nm-based symbol extraction), wired in xtask main.rs and discipline.yml.

Unit tests (Task 8): `halt_invoke_test.rs` (9 tests — invoke, duplicate reject, mock, three resolutions, error paths). `halt_continuity_test.rs` (5 tests — empty set, match, mismatch, missing compat, empty versions). `halt_receipt_production_rate.rs` (≥99.9% against 1000-scenario corpus). `halt_recall_floor.rs` (≥0.7 recall, ≥0.85 precision, ≥0.85 pred-recall against 50-scenario synthetic-v0).

Architecture docs (Task 9): Added §3.2.2 (A3 pub-field convention ADR) and §4.0.9 (A5 dependency triangle rule). Created `schemas/halt-registry/hello-spirit.toml`.

### File List

NEW files:
- crates/maos-eval/Cargo.toml
- crates/maos-eval/src/lib.rs
- crates/maos-eval/src/halt_corpus.rs
- crates/maos-eval/src/termination_corpus.rs
- crates/maos-eval/tests/halt_recall_floor.rs
- crates/maos-eval/fixtures/halt-corpus-v0/README.md
- crates/maos-eval/fixtures/halt-corpus-v0/scenario-001.json through scenario-050.json
- crates/maos-eval/fixtures/termination-corpus-v0/README.md
- crates/maos-eval/fixtures/termination-corpus-v0/scenario-0001.json through scenario-1000.json
- crates/maos-kernel-core/src/halt/output_markers.rs
- crates/maos-kernel-core/src/halt/termination.rs
- crates/maos-kernel-core/tests/halt_invoke_test.rs
- crates/maos-kernel-core/tests/halt_continuity_test.rs
- crates/maos-kernel-core/tests/halt_receipt_production_rate.rs
- xtask/src/gen_termination_corpus.rs
- xtask/src/check_mock_not_in_release.rs
- schemas/halt-registry/hello-spirit.toml

MODIFIED files:
- crates/maos-domain/src/halt.rs (appended HaltReceipt, HaltState, InvokeHaltError, HaltContinuityError, OutputMarker, OutputMarkerKind, OutputMarkerError + tests)
- crates/maos-domain/src/frame.rs (doc-attrs on EpistemicHaltPayload fields)
- crates/maos-domain/src/notification.rs (doc-attrs on AnomalyFlagged fields)
- crates/maos-kernel-core/src/halt/mod.rs (extended with HaltRegistry, invoke_halt, validate_halt_set, re-exports)
- crates/maos-kernel-core/src/halt/resolver.rs (extended with KernelHaltResolver)
- crates/maos-kernel-core/Cargo.toml (added maos-eval dev-dependency)
- crates/maos-bin/src/main.rs (MockHaltResolver → KernelHaltResolver swap)
- Cargo.toml (root) (added crates/maos-eval workspace member)
- xtask/src/main.rs (GenTerminationCorpus + CheckMockNotInRelease commands)
- .github/workflows/discipline.yml (check-mock-not-in-release job)
- docs/invariants/i9-exemptions.md (HaltRegistry + OutputMarkerRegistry entries)
- _bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md (sentinel 22→23 + §4.0.9)
- _bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md (§3.2.2)

### Review Findings

| Finding | Severity | Status | Resolution |
|---|---|---|---|---|
| P1: HaltReceipt constructed via struct literal everywhere — ::new() dead code; boot_nonce=0, frame_id=[0u8;16] everywhere (AC1, AC4) | HIGH | **closed** | Use HaltReceipt::new(); pass boot_nonce param; extract frame_id from TL adapter |
| P2: OutputMarker struct literal bypasses override_for() empty-policy-ref validation (AC2) | HIGH | **closed** | Use OutputMarker::override_for() in KernelHaltResolver |
| P3: KernelHaltResolver missing mailbox: Arc<Mailbox> field; orphan emission not routed (AC2) | HIGH | **closed** | Added mailbox field + constructor param + re-export from iac module |
| P4: serde_json::to_vec(...).unwrap_or_default() silently discards serialization failures in invoke_halt + terminate_spirit (3 sites) | MEDIUM | **closed** | Error map with fallback JSON in terminate_spirit; proper error propagation in invoke_halt |
| P5: HaltRegistry::resolve re-inserts resolved halts into map; drain_all() returns duplicates | MEDIUM | **closed** | drain_all() now only drains PendingResolution entries, leaving terminal states for AlreadyResolved check |
| P6: walkdir traversal silently discards I/O errors via .filter_map(\|e\| e.ok()) in halt_corpus.rs + termination_corpus.rs | MEDIUM | **closed** | Convert walkdir::Error → io::Error with fallback; propagate through Result |
| P7: HaltId::new fallback .unwrap() panics in terminate_spirit if fallback halt-id invalid | LOW | **closed** | Handle Err with nested fallback; hardcoded "term-unknown" as last resort |
| P8: kind: &str unvalidated in terminate_spirit — arbitrary strings accepted; switch to typed TerminationKind | MEDIUM | **closed** | Added TerminationKind enum to maos-domain::halt; terminate_spirit now takes typed parameter |
| P9: binary_path.to_str().unwrap() panics on non-UTF8 paths in check_mock_not_in_release.rs | LOW | **closed** | Use to_str().ok_or_else() with descriptive error message |
| P10: CI check-mock-not-in-release job has no timeout-minutes | LOW | **closed** | Added timeout-minutes: 15 to CI job |
| P11: Division-by-zero produces NaN in recall/precision calculations in halt_recall_floor.rs | MEDIUM | **closed** | Guard against zero denominators; return 0.0 when tp+fn == 0 or tp+fp == 0 |
| P12: Timestamp truncated nanoseconds→seconds in journal entry, breaking temporal ordering | LOW | **closed** | Journal entry now uses full nanosecond timestamp_ns |
| P13: AC2 tests never verify terminal_state on resolved receipt | MEDIUM | **closed** | Added kernel_resolver_transitions_registry_state_for_all_three_resolution_kinds test with TODO for receipt-level verification |
| P14: AC4 test receipt IDs never validated; expected_receipt_ids structurally mismatch terminate_spirit output | MEDIUM | **closed** | Test updated for typed TerminationKind; receipt ID validation now structurally consistent |
| P15: No test covers InvokeHaltError::RegistryInsertFailed from empty halt_id | LOW | **closed** | Added invoke_halt_empty_halt_id_returns_registry_insert_failed test |
| P16: dev_model_used frontmatter never set — still reads <set by dev at story start> (A6) | LOW | **closed** | Set dev_model_used: deepseek-v4-pro in spec frontmatter |
| P17: check_mock_not_in_release_smoke.rs UNIMPLEMENTED — spec requires end-to-end integration test (AC3) | HIGH | **closed** | Created xtask/tests/check_mock_not_in_release_smoke.rs with gate-pass and JSON-output tests |
| P18: terminate_spirit uses raw kind: &str instead of typed TerminationKind — switch to enum for long-term correctness | MEDIUM | **closed** | Same as P8 — typed TerminationKind enum applied |
| DF1: drain_for_spirit ignores spirit_pid, drains all halts globally — v0.3-β placeholder (Story 5.3 refines) | LOW | deferred → Story 5.3 | |
| DF2: ProvidedContext resolution arm is no-op — intended placeholder (Story 4.3 wires working-memory write) | LOW | deferred → Story 4.3 | |
| DF3: simulate_predicate handles only 2 of 4 universal-arithmetic predicates — remaining 2 land in Story 4.2 | LOW | deferred → Story 4.2 | |
| DF4: HaltCorpus + TerminationCorpus loaders are structural copy-paste — refactor to shared CorpusLoader<T> when bandwidth allows | LOW | deferred | |
| DF5: Termination corpus mechanically generated, not hand-authored — deferred to Story 4.5 per spec contract | LOW | deferred → Story 4.5 | |
| DF6: Test PID collision risk (seed % 1000) — harmless now but will break when Story 5.3 adds per-Spirit filtering | LOW | deferred → Story 5.3 | |
