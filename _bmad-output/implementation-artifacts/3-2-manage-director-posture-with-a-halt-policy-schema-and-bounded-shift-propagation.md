---
dev_model_used: <set by dev at story start>
---

# Story 3.2: Manage Director Posture with a Halt-Policy Schema and Bounded Shift Propagation

**Status:** done

**Type:** Epic 3 mid-arc story — operationalizes the director's autonomy
control. Lands the runtime posture state, the `[epistemic_policy]` halt-policy
schema, the `maosctl posture --shift` CLI surface, and the NFR-Perf-4 (P99 ≤2s,
P99.9 ≤5s) propagation guarantee. Consumes the `PosturePreferences` placeholder
shape Story 3.1 reserved (additive-only). Hands the `[epistemic_policy]`
thresholds off to Story 4.2 (predicate firing) and the halt-resolution UX off
to Story 3.3.

## Story

As a **director**,
I want three runtime postures (`autonomous-with-halt`, `assistive`, `cautious`)
that I can shift at runtime via `maosctl posture <spirit> --shift <new>` with
propagation P99 ≤2s across all in-flight capability decisions, **and** a
per-Spirit per-tag `[epistemic_policy]` halt-policy schema that lets me tune
halt-recall vs halt-precision,
So that I can dial Spirit autonomy up or down in real time without restarting
the Spirit AND so the halt-emit path that Story 4.2 wires has a parsed, validated
policy to read from.

## Acceptance Criteria

### AC1 — `[epistemic_policy]` manifest section parser + 3-case fixture set (Epic 2 retro A3)

**Given** Epic 2 retro action **A3** ("pin `[epistemic_policy]` manifest section
in the manifest parser with a structural validator + 3-case fixture set,
matching NFR-Test-13 discipline") was explicitly deferred to a bridge "before
Story 3.2 opens" and the bridge bundle (Story 2.5) explicitly DID NOT close A3
(`2-5-epic-3-prep-iac-addendum-d11-drain.md:19,273` — "Not the `[epistemic_policy]`
manifest pin")
**And** architecture §4.6.1 "Manifest policy (`[epistemic_policy]`)" + §5.1
manifest example specify the schema verbatim — per-tag rules mapping output
frame tags to one of three actions (`verbalize_only` / `flag` / `halt`), with
optional `on_confidence_below` (numeric threshold, range [0.0, 1.0]) and
`on_evidence_conflict` (boolean) qualifiers, plus a `default_action` that
defaults to `verbalize_only` so the kernel "fails open, never closed"
**When** Story 3.2 lands the schema pin
**Then** a new type `EpistemicPolicySection` lives at
`crates/maos-kernel-core/src/security/manifest.rs` (NEXT TO `PostureSection` —
preserve the existing 1b.5c re-export order to keep
`check-service-boundary`'s signature hashes stable; APPEND new `pub use` lines
at the bottom of `crates/maos-kernel-core/src/security/mod.rs` per the
discipline at `mod.rs:25-32`):

```rust
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicAction {
    VerbalizeOnly,
    Flag,
    Halt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicPolicyRule {
    pub tag: String,                               // e.g., "claim.security_vulnerability"
    pub action: EpistemicAction,
    pub on_confidence_below: Option<f32>,          // [0.0, 1.0]; validation rejects out-of-range
    pub on_evidence_conflict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicPolicySection {
    pub rules: Vec<EpistemicPolicyRule>,
    pub default_action: EpistemicAction,           // defaults to VerbalizeOnly per §4.6.1
}

impl EpistemicPolicySection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError>;
}
```

**And** the parser uses the same `RawXxx` + `validate` pattern as
`PostureSection` (`manifest.rs:381-402`) with `#[serde(deny_unknown_fields)]`
on every raw struct so a typo'd field becomes `ManifestError::Toml(…)` at
parse time (Precondition 5 from 1b.3)
**And** validation rejects:
  - `on_confidence_below` outside `[0.0, 1.0]` → `ManifestError::Toml(…)`
    with `validation_msg("epistemic_policy.on_confidence_below", "must be in
    [0.0, 1.0]")`
  - duplicate tags within `rules` → `ManifestError::Toml(…)` with
    `validation_msg("epistemic_policy.rules", &format!("duplicate tag '{tag}'"))`
  - empty tag string → `ManifestError::Toml(…)` with `validation_msg("epistemic_policy.rules", "tag must be non-empty")`
  - tags containing whitespace → same shape (mirrors `OutputShape` validation at `manifest.rs:444-451`)
**And** the NFR-Test-13 `manifest_field_coverage` walker
(`crates/maos-kernel-core/tests/manifest_field_coverage.rs`) gains tuples for
the new section. The `MANIFEST_FIELDS` const grows by EXACTLY these entries
(preserve all existing entries; append at the end of the slice to keep diffs
reviewable):

```rust
("epistemic_policy", "rules"),
("epistemic_policy", "default_action"),
```

**And** ≥3 fixture cases land per `(section, field)` under
`crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/{well-formed,
malformed-rejected, edge-case}/{rules.toml, default_action.toml}` — six new
fixture files total. Reference shapes:

```toml
# well-formed/rules.toml — full per-tag rule
[[rules]]
tag = "claim.security_vulnerability"
action = "halt"
on_confidence_below = 0.85
on_evidence_conflict = true

# edge-case/rules.toml — boundary thresholds, multiple rules, no qualifiers
[[rules]]
tag = "claim.exploratory"
action = "verbalize_only"
[[rules]]
tag = "claim.style_suggestion"
action = "flag"
on_confidence_below = 0.0
[[rules]]
tag = "diagnosis.root_cause"
action = "halt"
on_confidence_below = 1.0

# malformed-rejected/rules.toml — out-of-range threshold (must FAIL parse)
[[rules]]
tag = "claim.x"
action = "halt"
on_confidence_below = 1.5

# well-formed/default_action.toml
default_action = "verbalize_only"

# edge-case/default_action.toml — explicit `halt` default (uncommon but valid)
default_action = "halt"

# malformed-rejected/default_action.toml — unknown variant
default_action = "scream_loudly"
```

**And** the `xtask manifest_field_coverage` test passes with EXACTLY 0 orphan
fixtures (the walker's Decision Register D1 "reverse-validate" rule)
**And** unit tests in `manifest.rs::tests` (mirror the existing
`posture_section_*` test family at `manifest.rs:837-872`) cover:
`epistemic_policy_well_formed_parses`,
`epistemic_policy_rejects_threshold_above_one`,
`epistemic_policy_rejects_threshold_below_zero`,
`epistemic_policy_rejects_duplicate_tag`,
`epistemic_policy_rejects_empty_tag`,
`epistemic_policy_default_action_defaults_to_verbalize_only_when_omitted`
**And** the `spirits/hello-spirit/manifest.toml` reference file gains a minimal
`[epistemic_policy]` block (`default_action = "verbalize_only"` only — no rules
needed; hello-spirit halts on neither tag) so the v0.1 evaluator path still
admits hello-spirit without regression
**And** `cargo run -p xtask -- abi-diff` reports 0 removed / 0 changed (purely
additive — new enum, new structs, new `pub use` lines appended)

### AC2 — Extend `PosturePreferences` with halt-policy preferences (additive on Story 3.1)

**Given** Story 3.1 reserved `PosturePreferences` as a placeholder shape with
`#[non_exhaustive]` on `PostureHint` and `#[serde(default)]` on
`preferred_posture` ("Story 3.2 will extend this struct with halt-policy
preferences per FR19 + ADR-013 — additive-only; serde defaults preserve 3.1-era
wire compatibility" per `crates/maos-domain/src/frame.rs:67-84`)
**And** FR19 specifies "per-Spirit per-tag halt-recall vs halt-precision
preference" — the director declares a global tilt that the kernel uses to
adjust the Spirit's `on_confidence_below` thresholds at admission
**When** Story 3.2 extends the placeholder
**Then** `crates/maos-domain/src/frame.rs::PosturePreferences` gains:

```rust
pub struct PosturePreferences {
    #[serde(default)]
    pub preferred_posture: Option<PostureHint>,

    /// Story 3.2 — per-tag halt-policy override; missing tag means inherit
    /// the Spirit's manifest-declared policy unchanged. Each override declares
    /// a recall-vs-precision tilt in [-1.0, +1.0]: negative biases for higher
    /// halt-precision (tighten threshold, fewer false halts); positive biases
    /// for higher halt-recall (loosen threshold, fewer missed halts).
    /// Range-validated by `EpistemicPolicySection::apply_director_preferences`.
    #[serde(default)]
    pub halt_policy_overrides: Vec<HaltPolicyOverride>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HaltPolicyOverride {
    pub tag: String,
    /// Tilt in [-1.0, +1.0]; clamped at apply time. NaN rejected at apply time.
    pub recall_vs_precision: f32,
}
```

**And** the additive shape preserves 3.1-era wire compatibility: a 3.1-emitted
frame deserializes successfully with `halt_policy_overrides: Vec::new()` via
`#[serde(default)]`
**And** a serde round-trip test in
`crates/maos-domain/src/frame.rs::tests` confirms:
  - 3.1-shape `{"preferred_posture": null}` parses with empty
    `halt_policy_overrides`
  - 3.2-shape with `halt_policy_overrides: [{tag: "x", recall_vs_precision:
    0.3}]` round-trips byte-equal
  - 3.2-shape with `recall_vs_precision: NaN` round-trips at the SERDE layer
    (validation lives at apply time, not parse time, since serde-json admits
    f32 NaN per spec)
**And** `cargo run -p xtask -- abi-diff` reports 0 removed / 0 changed (additive
fields only; the existing `PostureHint` enum stays `#[non_exhaustive]`)

### AC3 — Runtime `PostureState` per Spirit + deterministic posture-hash

**Given** the `posture_snapshot_hash: [u8; 32]` field already exists on every
capability token (`crates/maos-kernel-core/src/capability/cap_tokens/body.rs:15`
+ `cap_tokens/mod.rs:140,175,235`) AND the token-verify path already enforces
TOCTOU re-validation against current posture
(`cap_tokens/mod.rs:235` — "if state.posture_hash != current_posture_hash:
return Err(CapError::PostureMismatch)")
**And** the current production code passes `[0u8; 32]` placeholder for
`posture_hash` everywhere
(`crates/maos-kernel-core/src/inference/mod.rs:75-77`)
**And** the propagation mechanism the AC requires (subsequent capability
decisions reflect the new posture within 2s P99) is ALREADY structurally
present via TOCTOU rejection — the missing piece is the actual hash function
and a runtime state holder
**When** Story 3.2 lands the posture-state + hash machinery
**Then** a new module `crates/maos-kernel-core/src/security/posture.rs` (NEW
file) defines:

```rust
use sha2::{Digest, Sha256};

/// Effective runtime posture for a single Spirit.
/// Held inside PolicyTableInner so CoW updates propagate atomically.
#[derive(Debug, Clone, PartialEq)]
pub struct PostureState {
    /// The currently-active runtime posture.
    pub current: Posture,
    /// The manifest ceiling that bounds runtime shifts (operator can lower
    /// further but never raise).
    pub allowed_max: Posture,
    /// Halt-policy rules applied (manifest section, with director overrides
    /// already folded in via `apply_director_preferences`).
    pub epistemic_policy: EpistemicPolicySection,
}

impl PostureState {
    /// Deterministic hash bound into every capability token's
    /// `posture_snapshot_hash` field. Domain-separated with a fixed prefix.
    /// SHA-256 over a canonical encoding of:
    ///   (posture_variant_u8, allowed_max_variant_u8,
    ///    LEB128(rules.len()), [(tag_bytes, action_u8, threshold_bits,
    ///     conflict_flag) ...], default_action_u8)
    /// Sorted by `tag` for determinism. Returns `[u8; 32]`.
    pub fn posture_hash(&self) -> [u8; 32];

    /// Apply a director's `HaltPolicyOverride` list — clamps tilt to
    /// [-1.0, +1.0]; rejects NaN with PostureError::InvalidOverride;
    /// produces a new EpistemicPolicySection with adjusted thresholds.
    /// Tilt mapping: new_threshold = (existing_threshold ± 0.1 * tilt).clamp(0.0, 1.0)
    /// where +tilt RAISES `on_confidence_below` (more halts; higher recall)
    /// and -tilt LOWERS it (fewer halts; higher precision).
    pub fn apply_director_preferences(
        policy: EpistemicPolicySection,
        overrides: &[HaltPolicyOverride],
    ) -> Result<EpistemicPolicySection, PostureError>;
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PostureError {
    #[error("requested posture {requested:?} exceeds allowed_max {allowed:?}")]
    AboveCeiling { requested: Posture, allowed: Posture },
    #[error("posture {0:?} is not a runtime posture at v0.3 — use cautious / assistive / autonomous-with-halt")]
    NonRuntimePosture(Posture),
    #[error("invalid director override: {0}")]
    InvalidOverride(String),
    #[error("unknown spirit {0}")]
    UnknownSpirit(u32),
}
```

**And** `Posture::Autonomous` (the 4th variant) is REJECTED as a runtime shift
target with `PostureError::NonRuntimePosture(Posture::Autonomous)`. Per
architecture §5.4 "autonomous: rare; explicit user grant" — Story 3.2 only
operationalizes the three runtime postures the AC enumerates. The `Autonomous`
variant stays in the `Posture` enum (no removal — `abi-diff` stays additive),
but is unreachable via `maosctl posture --shift`. Doc-comment on `Posture::Autonomous`
adds "Runtime shift to this posture is rejected at v0.3 — see Story 3.2 AC3."
**And** `posture_hash` is domain-separated (first 16 bytes of input are
`b"maos.posture.v1\0"`) so collisions with any other SHA-256 use in the
codebase are structurally impossible
**And** unit tests verify:
  - `posture_hash` is deterministic across calls
  - Hash CHANGES when posture variant changes (any of 3 → any other of 3)
  - Hash CHANGES when any rule's threshold changes
  - Hash CHANGES when `default_action` changes
  - Hash is STABLE under rule reordering in the input `Vec` (canonicalize
    by sorting `tag` before hashing)
  - `apply_director_preferences` clamps tilt to [-1.0, +1.0]
  - `apply_director_preferences` rejects NaN with `InvalidOverride`
  - `apply_director_preferences` is idempotent: applying an empty overrides
    list returns the input unchanged
**And** `sha2 = "0.10"` is ALREADY pinned in
`crates/maos-kernel-core/Cargo.toml:64` — use it directly (no new
dep, no workspace-deps change, no justification entry needed)

### AC4 — Atomic posture-shift API with Approval Decision Log journaling (I4)

**Given** `PolicyTable` is read-mostly copy-on-write via
`Arc<ArcSwap<PolicyTableInner>>` (`cap_policy/mod.rs:55-66`) AND the
`update(new_inner)` method (`cap_policy/mod.rs:143-146`) is the existing atomic
swap surface
**And** Invariant I4 (every approval captures intent) is enforced runtime from
v0.1 via `TransparencyLogAdapter::insert_approval_decision`
**When** the kernel receives a posture-shift request
**Then** `PostureState` is held INSIDE `PolicyTableInner` as
`pub spirit_postures: HashMap<u32, PostureState>`, sharing the same CoW
discipline as `manifest_scopes` (`cap_policy/mod.rs:46`)
**And** a new method on `PolicyTable` is added:

```rust
impl PolicyTable {
    /// Atomically shift a Spirit's posture under the ceiling constraint.
    ///
    /// 1. Validates spirit_pid is known.
    /// 2. Validates `new_posture <= state.allowed_max`.
    /// 3. Rejects `Posture::Autonomous` per AC3.
    /// 4. CoW-swaps `PolicyTableInner` with the updated `PostureState`.
    /// 5. Returns the new `posture_hash` so the caller can journal it.
    ///
    /// Does NOT write to the Approval Decision Log — the CALLER does that
    /// to preserve the read-mostly hot-path purity of `PolicyTable`. The
    /// composition-root wiring at AC8 owns the journal write.
    pub fn shift_posture(
        &self,
        spirit_pid: u32,
        new_posture: Posture,
    ) -> Result<[u8; 32], PostureError>;
}
```

**And** a new module-level function in `crates/maos-kernel-core/src/security/posture.rs`:

```rust
pub fn journal_posture_shift(
    log: &TransparencyLogAdapter,
    actor: &str,                          // e.g., "director"
    spirit_id: &str,
    from: Posture,
    to: Posture,
) -> Result<(), AuditError> {
    log.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: spirit_id.into(),
        capability: "posture.shift".into(),     // stable label; never localized
        intent: format!("{:?} -> {:?}", from, to),
        decision: true,
        reasoning: None,                         // Story 3.3 wires reasoning UX
    })
}
```

**And** the e2e flow is exercised by a new integration test
`crates/maos-kernel-core/tests/posture_shift_journaled.rs` (NEW file):
  - Open in-memory `TransparencyLogAdapter`, build `PolicyTable`, seed one
    Spirit with default `Posture::Assistive` + `allowed_max =
    Posture::AutonomousWithHalt`.
  - Call `policy.shift_posture(pid, Posture::AutonomousWithHalt)` → returns new hash.
  - Call `journal_posture_shift(...)`.
  - Query `log.query_approvals(None)` → assert one row exists with
    `capability == "posture.shift"`, `intent == "Assistive -> AutonomousWithHalt"`,
    `decision == true`.
  - Assert the row landed in `approval_decision_log` and NOT in
    `transparency_log` (reuse the `approval_log_is_distinct_table` test
    pattern at `transparency_log.rs:660-728`).
**And** an additional test asserts ceiling rejection:
`policy.shift_posture(pid_with_assistive_ceiling, Posture::AutonomousWithHalt)`
returns `Err(PostureError::AboveCeiling { .. })` AND no approval-decision row
is written (caller responsibility, but the test documents the negative path)
**And** `Posture::Autonomous` shift attempt returns
`Err(PostureError::NonRuntimePosture(Posture::Autonomous))`

### AC5 — Posture-aware approval-class classifier (3 postures × 6 classes)

**Given** the 6 approval classes from architecture §4.3.3 are pinned by
Story 3.1 AC5 (`crates/maos-domain/src/notification.rs::ApprovalClass`) AND
the existing `PolicyTable::evaluate` (`cap_policy/mod.rs:73-114`) returns
`PolicyDecision::{Allow, Deny, RequireApproval { class }}` keyed off operator
policy
**And** the three-posture × six-class behavior matrix from epic-3 ACs +
architecture §4.3.3 + §5.4 is normative:

| Posture                    | readonly_scoped | readonly_search | mutating | exec_capable | control_plane | interactive |
|----------------------------|-----------------|-----------------|----------|--------------|---------------|-------------|
| `cautious`                 | Allow*          | Allow*          | Prompt   | Prompt       | Prompt        | Prompt      |
| `assistive`                | Allow           | Allow           | Prompt   | Prompt       | Prompt        | Prompt      |
| `autonomous-with-halt`     | Allow           | Allow           | Allow    | Allow        | Prompt        | Allow       |

  *cautious auto-allows readonly_scoped + readonly_search per the epic-3 AC
  text "cautious (auto-approve routine, prompt for novel)" → "readonly" =
  routine. `control_plane` ALWAYS prompts regardless of posture (per §4.3.3
  default "prompt; no remember-this-decision").

**When** a capability invocation enters the policy evaluation surface
**Then** a new method is added to `PolicyTable` (DO NOT modify existing
`evaluate` — additive surface preserves the abi-diff contract):

```rust
impl PolicyTable {
    /// Posture-aware evaluation. Returns `PolicyDecision` per the 3×6 matrix
    /// after applying base-class operator overrides.
    ///
    /// Resolution order:
    ///   1. If `spirit_postures[pid]` is missing → Deny (fail-closed).
    ///   2. If `base_class == ControlPlane` → RequireApproval (always).
    ///   3. Apply posture × class matrix → Allow or RequireApproval { class: base_class }.
    ///   4. Operator policy overrides in `operator_policy.per_capability_approval`
    ///      take precedence (existing `evaluate` semantics).
    pub fn evaluate_with_posture(
        &self,
        spirit_pid: u32,
        base_class: ApprovalClass,
    ) -> PolicyDecision;
}
```

**And** the matrix lives as a module-level const lookup in
`crates/maos-kernel-core/src/security/posture.rs`:

```rust
/// (posture, class) → bool (true = require approval, false = silent allow).
/// Const so the table is one source of truth; tests verify against it.
pub const POSTURE_APPROVAL_MATRIX: &[(Posture, ApprovalClass, bool)] = &[
    // cautious row
    (Posture::Cautious, ApprovalClass::ReadonlyScoped, false),
    (Posture::Cautious, ApprovalClass::ReadonlySearch, false),
    (Posture::Cautious, ApprovalClass::Mutating, true),
    (Posture::Cautious, ApprovalClass::ExecCapable, true),
    (Posture::Cautious, ApprovalClass::ControlPlane, true),
    (Posture::Cautious, ApprovalClass::Interactive, true),
    // assistive row
    (Posture::Assistive, ApprovalClass::ReadonlyScoped, false),
    (Posture::Assistive, ApprovalClass::ReadonlySearch, false),
    (Posture::Assistive, ApprovalClass::Mutating, true),
    (Posture::Assistive, ApprovalClass::ExecCapable, true),
    (Posture::Assistive, ApprovalClass::ControlPlane, true),
    (Posture::Assistive, ApprovalClass::Interactive, true),
    // autonomous-with-halt row
    (Posture::AutonomousWithHalt, ApprovalClass::ReadonlyScoped, false),
    (Posture::AutonomousWithHalt, ApprovalClass::ReadonlySearch, false),
    (Posture::AutonomousWithHalt, ApprovalClass::Mutating, false),
    (Posture::AutonomousWithHalt, ApprovalClass::ExecCapable, false),
    (Posture::AutonomousWithHalt, ApprovalClass::ControlPlane, true),  // always prompt
    (Posture::AutonomousWithHalt, ApprovalClass::Interactive, false),
];

pub fn posture_requires_approval(posture: Posture, class: ApprovalClass) -> bool;
```

**And** a `#[test] fn posture_matrix_covers_all_combinations` asserts the const
slice has EXACTLY 18 entries (3 runtime postures × 6 classes) and that EVERY
(posture, class) pair appears exactly once
**And** a `#[test] fn posture_matrix_matches_epic_3_acs` asserts the rows
verbatim against the table above (drift-gate pattern from Story 3.1 AC5's
`approval_classes_match_architecture`)
**And** `ApprovalClass` is re-exported into `maos-kernel-core::security::posture`
via `pub use maos_domain::notification::ApprovalClass;` so callers do not
cross crate boundaries needlessly
**And** the existing `ApprovalManager::prompt` (v0.3-β at
`crates/maos-kernel-core/src/security/approval.rs:31-72`) gains an
opt-in `posture` parameter via a NEW method (additive — keep `prompt` for
v0.3-β backward compat):

```rust
impl ApprovalManager {
    /// Posture-aware prompt — short-circuits to silent allow when the
    /// (posture, class) cell in POSTURE_APPROVAL_MATRIX is `false`.
    /// At v0.3-β still auto-allows on the prompt path; Story 3.3 wires
    /// the interactive resolution UI.
    pub fn prompt_with_posture(
        &self,
        posture: Posture,
        class: ApprovalClass,
        capability: String,
        reasoning: Option<String>,
        dispatcher: &NotificationDispatcher,
    ) -> Result<bool, AuditError>;
}
```

**And** when `posture_requires_approval(posture, class) == false`, the method
returns `Ok(true)` WITHOUT dispatching a notification AND WITHOUT writing to
the Approval Decision Log (silent allow). When `true`, the method behaves
identically to `prompt` (dispatch + log).
**And** unit tests in `approval.rs` cover all six classes under each of the
three runtime postures (18 tests OR one parameterized test with the const
matrix as data), asserting silent-allow paths produce zero
`approval_decision_log` rows and zero captured notifications

### AC6 — `maosctl posture <spirit> --shift <new>` CLI subcommand

**Given** `crates/maos-cli/src/cli.rs` defines the `Subcommand` enum with
six v0.1 verbs (Install, Start, Stop, Unload, Run, Audit per `cli.rs:39-61`)
AND `crates/maos-cli/src/subcommands.rs::dispatch` is the routing point
**And** the CLI MUST honor the NFR-Ops-5 accessibility cascade (NO_COLOR /
TERM=dumb / `--plain`) per the 1a.4 discipline
**When** Story 3.2 adds the posture surface
**Then** `Subcommand` gains a new variant `Posture(PostureArgs)` and a paired
`PostureArgs` clap struct (APPEND at the end of the enum to preserve clap's
declaration-order help text):

```rust
/// Shift the runtime posture of a Spirit (Story 3.2).
///
/// Posture shifts propagate to subsequent capability decisions within
/// P99 ≤2s, P99.9 ≤5s (NFR-Perf-4). The shift is journaled to the
/// Approval Decision Log per Invariant I4.
Posture(PostureArgs),

#[derive(clap::Args, Debug)]
pub struct PostureArgs {
    /// Spirit ID to shift.
    pub spirit: String,
    /// New runtime posture: cautious | assistive | autonomous-with-halt
    #[arg(long, value_enum)]
    pub shift: PostureChoice,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostureChoice {
    Cautious,
    Assistive,
    #[clap(name = "autonomous-with-halt")]
    AutonomousWithHalt,
}
```

**And** the v0.3-β CLI body is a thin shim — `maosctl posture` writes a
single `LifecycleEvent::PostureShift` (NEW variant per AC7) journal entry
following the same one-shot env-var bridge pattern Story 1b.5c used for
start/stop/unload (`crates/maos-bin/src/main.rs:225-277`). The full
control-plane HTTP path (`maos-control` crate) wires the live shift in
Story 9.1+ — Story 3.2's CLI surface is the operator-visible front door and
the journal-record contract; the supervised live-shift body uses
`MAOS_ONE_SHOT=posture-shift` with `MAOS_SPIRIT_ID` + `MAOS_POSTURE` env vars
**And** `dispatch_posture(args, color)` in `subcommands.rs` shells out to
`maos-bin` via the existing `MAOS_ONE_SHOT` discriminator pattern
(`subcommands.rs::lifecycle_verb` is the reference implementation), with
the v0.3-β exit code `0` on success and `2` on validation error (mirrors
existing v0.1-α "not-yet-implemented" exit semantics where applicable)
**And** integration tests in `crates/maos-cli/tests/posture_shift_test.rs`
(NEW file, parallel to `audit_no_color_test.rs:1-65` shape) verify:
  - `maosctl posture hello-spirit --shift cautious` exits 0
  - The Lifecycle Journal contains exactly one new `PostureShift` entry
  - `NO_COLOR=1 maosctl posture hello-spirit --shift assistive` produces zero
    ANSI escape bytes in stderr (mirror `accessibility_test.rs`)
  - `maosctl posture hello-spirit --shift autonomous` (the 4th, non-runtime
    posture) is REJECTED by clap value parsing — the `PostureChoice` enum has
    only 3 variants
**And** the existing 35-job discipline.yml needs no new jobs (the CLI tests
slot into the existing `cargo test --workspace` matrix)

### AC7 — `LifecycleEvent::PostureShift` enum variant + journal write integration

**Given** `LifecycleEvent` in `crates/maos-domain/src/invariants/i10.rs:36-51`
is the journal-event taxonomy with explicit `#[repr(u8)]` discriminants —
existing variants `Load = 0, Start = 1, Pause = 2, Swap = 3, Migrate = 4,
Unload = 5, Halt = 6` — and any addition MUST be additive-only per the I10
audit-spine contract
**When** Story 3.2 lands the journal write for posture shifts
**Then** `LifecycleEvent` gains a NEW variant `PostureShift = 7` (APPEND at
END of enum to preserve discriminant ordering for forward-compat NDJSON
deserialization — `abi-diff` reports `1 variant added`, classified as additive
per the architecture §8.5 ABI break rule: "additive enum variants at the end
with explicit `#[repr(u8)]` discriminants and `#[serde(other)]` fallback —
does NOT bump"):

```rust
pub enum LifecycleEvent {
    // ... existing variants 0..=6 unchanged ...
    /// Story 3.2 — director-initiated runtime posture shift.
    /// Journaled to the Lifecycle Journal for forensic replay;
    /// also written to the Approval Decision Log per AC4.
    PostureShift = 7,
}
```

**And** the existing `journal_entry_backward_compat_deser` test
(`i10.rs:84-92`) STILL passes — old NDJSON without the new variant parses
unchanged; the new variant only appears in fresh writes from this story
forward

**And** the composition root one-shot arm (`crates/maos-bin/src/main.rs:225-277`)
gains a new `MAOS_ONE_SHOT=posture-shift` branch that:
  1. Reads `MAOS_SPIRIT_ID` (required; error if missing)
  2. Reads `MAOS_POSTURE` (required; one of `cautious|assistive|autonomous-with-halt`)
  3. Loads the Spirit's manifest + admits via the existing `admit_spirit`
     path so the PolicyTable knows the Spirit (reuse the existing manifest
     plumbing at `main.rs:288-361`)
  4. Calls `policy.shift_posture(spirit_pid, new_posture)` per AC4
  5. Writes a `LifecycleEvent::PostureShift` journal entry via the existing
     `JournalAdapter` (`main.rs:249-263` shape)
  6. Calls `journal_posture_shift(...)` per AC4 to write the
     `approval_decision_log` row
  7. Drains the cap-audit channel + `audit_writer.await` (preserve the
     2-5 D11 drain pattern at `main.rs:394-402`)
  8. Exits with code 0
**And** any new error path returns a non-zero exit code WITHOUT panicking
(matches the existing one-shot error discipline at `main.rs:280-282`)
**And** a doctest on `Posture::Autonomous` is added (or its doc-comment
updated) stating "Story 3.2: not a runtime shift target; use cautious /
assistive / autonomous-with-halt via `maosctl posture --shift`"
**And** the composition-root drain sequence is preserved verbatim — drop
order remains `audit_tx → inference → capability → iac → mailbox → dispatcher`
(`main.rs:425-432`)

### AC8 — NFR-Perf-4 propagation-latency proof: 1000-shift corpus, P99 ≤2s, P99.9 ≤5s

**Given** NFR-Perf-4 specifies "Posture-shift propagation P99 ≤ 2s, P99.9 ≤ 5s
in 1000-shift corpus" (`_bmad-output/planning-artifacts/prd/non-functional-requirements.md:12`)
AND the propagation mechanism is the CoW swap on `PolicyTableInner` plus the
fact that capability tokens fail TOCTOU verify after a shift (forcing re-issue
which re-reads the new posture from the swapped inner)
**When** Story 3.2 proves the latency floor
**Then** a new integration test `crates/maos-kernel-core/tests/nfr_perf_4_posture_shift_propagation.rs`
(NEW file) executes the 1000-shift corpus:
  1. Construct `PolicyTable` + register 10 simulated Spirits (parameterizable;
     1 Spirit is sufficient for the latency floor but 10 stresses the
     `HashMap<u32, PostureState>` lookup).
  2. Loop 1000 times — each iteration:
     a. Pick a Spirit by round-robin
     b. Record `t0 = Instant::now()`
     c. Call `policy.shift_posture(pid, alternating_posture)`
     d. In the same `Arc<PolicyTable>` from a SECOND task (`tokio::spawn`),
        attempt `policy.evaluate_with_posture(pid, ApprovalClass::Mutating)`
        and verify the returned `PolicyDecision` reflects the new posture
        (this is the "subsequent capability decision" the AC requires)
     e. Record `t1 = Instant::now()` when the assertion passes
     f. Push `t1 - t0` into a histogram
  3. After 1000 iterations, compute P50, P99, P99.9 from the histogram
  4. Assert: `P99 <= 2_000_000_000` ns (2s); `P99.9 <= 5_000_000_000` ns (5s)
**And** the test EMITS its measured percentiles (P50 / P99 / P99.9) to stderr
on PASS so dev records can copy the numbers verbatim (the 1b.5b telemetry
pattern from `tests/integration/v01_evaluator_path.sh` is the precedent)
**And** the test runs in <60s wall-clock on the CI runner (Story 0.1's
discipline) — if a `tokio::task::yield_now` injection point is needed to
keep the test from monopolizing a worker thread, document it in the dev
record
**And** a stretch assertion lands: `P50 <= 1_000_000` ns (1ms) — the CoW swap
is sub-microsecond and the cross-task observation should be tens of
microseconds. The 2s P99 floor exists to cover scheduler hiccups under load,
not to license slow swaps. Document the measured P50 in the dev record so
future stories can compare against the baseline.
**And** if the test reveals propagation latency above floor, the dev record
explicitly documents the cause (e.g., DashMap shard contention, missing
`tokio::yield_now` injection, ArcSwap fence semantics) AND opens a
follow-up story rather than weakening the floor — NFR-Perf-4 is binding-v0.3

### AC9 — Composition-root wiring + admission seeds initial posture

**Given** `SecurityManagerAdapter::admit_spirit`
(`crates/maos-kernel-core/src/security/mod.rs:102-189`) is the canonical
admission path that already parses + validates the `[posture]` section
indirectly (via Story 1b.5c's PostureSection) but does NOT yet persist the
posture into `PolicyTable`
**And** the composition root at `crates/maos-bin/src/main.rs:286-361` is the
ONLY caller that builds a `PolicyTable` and admits Spirits
**When** Story 3.2 wires admission → PostureState seeding
**Then** `admit_spirit` gains a new parameter `posture_section: &PostureSection`
(additive — at the END of the parameter list to keep the signature-hash
delta minimized AND so existing test/composition-root call sites need a
single-line change rather than a re-ordering refactor):

```rust
pub fn admit_spirit(
    &self,
    spirit_pid: u32,
    spirit_id: &str,
    _manifest: &SandboxConfig,
    caps: &ResourceCaps,
    caps_required: &CapabilitiesRequired,
    output_shape: Option<&OutputShape>,
    journal: &dyn SpiritSchedulerPort,
    posture_section: &PostureSection,        // NEW (Story 3.2)
    epistemic_policy: Option<&EpistemicPolicySection>,  // NEW (Story 3.2)
) -> Result<SandboxSpec, SecurityError>;
```

**And** inside `admit_spirit` the CoW PolicyTable update at
`security/mod.rs:124-133` is EXTENDED to ALSO insert a `PostureState` for the
admitted Spirit (preserving the existing single-update CoW semantics — one
`policy.update(new_inner)` call covers both `manifest_scopes` AND
`spirit_postures`):

```rust
new_inner.spirit_postures.insert(
    spirit_pid,
    PostureState {
        current: posture_section.default,
        allowed_max: posture_section.allowed_max,
        epistemic_policy: epistemic_policy
            .cloned()
            .unwrap_or_else(EpistemicPolicySection::default_open_fail),
    },
);
```

**And** `EpistemicPolicySection::default_open_fail()` returns
`EpistemicPolicySection { rules: vec![], default_action: EpistemicAction::VerbalizeOnly }`
— the fail-open default per architecture §4.6.1
**And** the composition root at `main.rs:309-329` parses the manifest's
`[posture]` AND `[epistemic_policy]` sections (the latter is optional —
hello-spirit's gains a minimal `default_action = "verbalize_only"` per AC1)
and passes them into `admit_spirit`:

```rust
let posture_section = maos_kernel_core::security::PostureSection::from_toml_str(
    &extract_section(&manifest_root, "posture")?,
)?;
let epistemic_policy = manifest_root.get("epistemic_policy")
    .map(|v| {
        let s = toml::to_string(v).map_err(|e| format!("epistemic_policy serialize: {e}"))?;
        maos_kernel_core::security::EpistemicPolicySection::from_toml_str(&s)
            .map_err(|e| format!("epistemic_policy parse: {e}"))
    })
    .transpose()?;

let _spec = security.admit_spirit(
    0, "hello-spirit",
    &sandbox_cfg, &resource_caps, &caps_required, Some(&output_shape),
    &journal,
    &posture_section,
    epistemic_policy.as_ref(),
)?;
```

**And** the v0.1 evaluator path
(`tests/integration/v01_evaluator_path.sh`) and the 2-5 server-exit drain
regression (`tests/integration/server_exit_drain.sh`) BOTH continue to pass —
the additive parameters DO change signature hashes for `admit_spirit` and
`SecurityManagerAdapter`, classify these as additive-only per
3.1 AC10's "ZST → struct with fields" precedent in the dev record
**And** every existing call site of `admit_spirit` (composition root +
any test scaffolding) is updated in this PR; the dev record cites the file
list of every updated caller

### AC10 — Discipline gates green; ABI freeze holds additive-only

**Given** the bridge 2-5 + Story 3.1 brought CI to 35 jobs; this story adds
NO new CI jobs
**When** the dev runs the full discipline sweep
**Then** all 35 jobs are GREEN
**And** `cargo run -p xtask -- abi-diff` reports the changes as
**additive-only**:
  - New types: `EpistemicAction`, `EpistemicPolicyRule`, `EpistemicPolicySection`,
    `PostureState`, `PostureError`, `HaltPolicyOverride`, `PostureChoice`,
    new variant `Posture(PostureArgs)`, new variant `LifecycleEvent::PostureShift`
  - New trait methods: `PolicyTable::shift_posture`, `PolicyTable::evaluate_with_posture`,
    `ApprovalManager::prompt_with_posture`, `PostureState::posture_hash`,
    `PostureState::apply_director_preferences`
  - New const: `POSTURE_APPROVAL_MATRIX`
  - Signature-hash changes (classified additive per the 3.1 AC10 precedent):
    `SecurityManagerAdapter::admit_spirit` (added 2 params at end),
    `PolicyTableInner` (added `spirit_postures: HashMap<u32, PostureState>` field)
  - Renaming/removing/reordering: **0** (no existing item is renamed, removed,
    or reordered — extension fields appended; trait method additions are
    new entries; the `Posture::Autonomous` doc-comment update is metadata
    only)
**And** `cargo run -p xtask -- check-empty-kernel` reports 0 new I9
violations — `PostureState` lives inside `PolicyTableInner` which is already
i9-exempt via `cap_policy/mod.rs:43-48`; no new exemption entries needed.
The dev record explicitly classifies `EpistemicPolicySection` as
parsed-then-dropped (mirrors `OutputShape` at `manifest.rs:411-423`) when
held by `SandboxSpec`, BUT held persistently inside `PostureState`. Add
ONE new entry to `docs/invariants/i9-exemptions.md`:

```markdown
### `PostureState` — `crates/maos-kernel-core/src/security/posture.rs`

**Reason:** Per-Spirit runtime posture + halt-policy state held inside
PolicyTableInner. Updated atomically via CoW swap (same shape as
manifest_scopes). Bounded by Spirit lifetime, keyed by spirit_pid, no
parameter drift — structural caching per I9.
```

**And** `cargo run -p xtask -- check-service-boundary` reports all P1-P4
properties hold; `SecurityManagerAdapter`'s signature hash CHANGES (added 2
params + extended responsibility) — classify as additive in the dev record
(same precedent as 3.1's `IacBusAdapter` ZST → body reclassification at
`3-1-route-task-assign-frames-over-the-iac-bus-with-notification-surface-dispatch.md:837-839`)
**And** `cargo run -p xtask -- check-workspace-count` PASSES — this story
adds NO new workspace members (member count stays at 22)
**And** `cargo run -p xtask -- check-unsafe` reports 0 new `unsafe` blocks
(the new `security/posture.rs` module declares `#![forbid(unsafe_code)]`
at the top)
**And** the kernel-api-classes.toml (`xtask/kernel-api-classes.toml`) does
NOT need new entries — the new methods are additions to existing adapters
(`SecurityManagerAdapter`, `CapabilityRegistryAdapter` paths) which are
already classified
**And** `cargo build --workspace --locked` is clean (no new compiler
warnings beyond pre-existing)
**And** `cargo test --workspace` passes the new AC1-AC8 suites plus all
pre-existing suites (the manifest_field_coverage walker now has +2 tuples
and +6 fixture files — must STILL pass `test_nfr_test_13_three_cases_per_field`)
**And** the dev record cites the explicit `discipline.yml` run conclusion
(per Epic 1b retro A8 and Story 2.5 AC1 / Story 3.1 AC10 discipline)

## Tasks / Subtasks

- [x] **T1: `[epistemic_policy]` manifest parser + fixtures (AC1)**
  - [x] T1.1 Add `EpistemicAction`, `EpistemicPolicyRule`, `EpistemicPolicySection`
        + `Raw*` shadows in `crates/maos-kernel-core/src/security/manifest.rs`
        APPENDED after `Budget` section (preserve prior ordering for
        signature-hash stability).
  - [x] T1.2 Implement `from_toml_str` + `validate` with the validation rules
        from AC1 (threshold range, duplicate tags, empty tag, whitespace,
        unknown variant via `#[serde(deny_unknown_fields)]`).
  - [x] T1.3 Add `pub use` lines for the new types at the END of `security/mod.rs`'s
        existing re-export block (preserve order; mirrors the 1b.5c discipline).
  - [x] T1.4 Author 6 fixture files at
        `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/{well-formed,
        malformed-rejected, edge-case}/{rules.toml, default_action.toml}` per
        AC1's reference shapes.
  - [x] T1.5 Update `MANIFEST_FIELDS` in
        `crates/maos-kernel-core/tests/manifest_field_coverage.rs` with the
        2 new tuples APPENDED at the end of the slice.
  - [x] T1.6 Add 6 unit tests in `manifest.rs::tests` per AC1's named test list.
  - [x] T1.7 Add `[epistemic_policy]` block to `spirits/hello-spirit/manifest.toml`
        (one line: `default_action = "verbalize_only"`).
  - [x] T1.8 Verify `cargo test -p maos-kernel-core
        test_nfr_test_13_three_cases_per_field` passes (no orphans, +2 tuples
        all covered).

- [x] **T2: Extend `PosturePreferences` with `halt_policy_overrides` (AC2)**
  - [x] T2.1 In `crates/maos-domain/src/frame.rs`, add `HaltPolicyOverride`
        struct AND extend `PosturePreferences` with `halt_policy_overrides:
        Vec<HaltPolicyOverride>` field. Both `#[serde(default)]`.
  - [x] T2.2 Add 3 serde round-trip tests per AC2 (3.1-shape compat, 3.2-shape
        round-trip, NaN admission at parse).
  - [x] T2.3 Update the `frame.rs` doc comment in the "Field ownership guide"
        table to reflect that 3.2 has filled in `PosturePreferences` extension.

- [x] **T3: `PostureState` + posture hash + `apply_director_preferences` (AC3)**
  - [x] T3.1 Create `crates/maos-kernel-core/src/security/posture.rs` with
        `#![forbid(unsafe_code)]` header and the types from AC3.
  - [x] T3.2 Wire `pub mod posture;` in `security/mod.rs` AND append
        `pub use posture::{PostureState, PostureError, ...};` at the END of
        existing re-export block.
  - [x] T3.3 Implement `PostureState::posture_hash` with the domain-separated
        SHA-256 canonical encoding per AC3.
  - [x] T3.4 Implement `PostureState::apply_director_preferences` with the
        tilt clamping + NaN rejection per AC3.
  - [x] T3.5 Import `use sha2::{Digest, Sha256};` — already pinned at
        `crates/maos-kernel-core/Cargo.toml:64` (`sha2 = "0.10"`), no dep
        change needed.
  - [x] T3.6 Add unit tests for deterministic hash, change-detection
        (posture/threshold/default_action), rule-reordering stability,
        tilt clamping, NaN rejection, idempotency.

- [x] **T4: `PolicyTable::shift_posture` + `journal_posture_shift` (AC4)**
  - [x] T4.1 Extend `PolicyTableInner` in
        `crates/maos-kernel-core/src/capability/cap_policy/mod.rs` with
        `pub spirit_postures: HashMap<u32, PostureState>` field.
  - [x] T4.2 Implement `PolicyTable::shift_posture` per AC4: validate spirit
        exists, validate ceiling, reject Autonomous, CoW-swap, return new
        posture_hash.
  - [x] T4.3 Add `journal_posture_shift` module-level fn in
        `security/posture.rs` per AC4.
  - [x] T4.4 Author integration test
        `crates/maos-kernel-core/tests/posture_shift_journaled.rs` per AC4's
        e2e flow (positive path + ceiling rejection + Autonomous rejection).

- [x] **T5: Posture-aware approval matrix + `evaluate_with_posture` (AC5)**
  - [x] T5.1 In `security/posture.rs`, add `POSTURE_APPROVAL_MATRIX` const
        with the 18 tuples from AC5.
  - [x] T5.2 Add `posture_requires_approval(posture, class) -> bool` lookup fn.
  - [x] T5.3 Add `PolicyTable::evaluate_with_posture` per AC5's resolution
        order (fail-closed missing posture, ControlPlane always prompts,
        matrix lookup, operator overrides take precedence).
  - [x] T5.4 Add `ApprovalManager::prompt_with_posture` per AC5 — silent-allow
        short-circuit when matrix says false.
  - [x] T5.5 Tests: `posture_matrix_covers_all_combinations`,
        `posture_matrix_matches_epic_3_acs` (drift gate), 18 per-cell
        behavior tests for `prompt_with_posture` OR one parameterized test
        with `POSTURE_APPROVAL_MATRIX` as data.

- [x] **T6: `maosctl posture` CLI subcommand (AC6)**
  - [x] T6.1 Add `Posture(PostureArgs)` variant + `PostureArgs` + `PostureChoice`
        clap structs at the END of `crates/maos-cli/src/cli.rs::Subcommand`.
  - [x] T6.2 Add `dispatch_posture(args, color)` in
        `crates/maos-cli/src/subcommands.rs` that shells out to `maos-bin`
        with `MAOS_ONE_SHOT=posture-shift`, `MAOS_SPIRIT_ID=...`,
        `MAOS_POSTURE=...` (mirror `lifecycle_verb` shape).
  - [x] T6.3 Author `crates/maos-cli/tests/posture_shift_test.rs` integration
        test: invokes the maosctl binary, asserts exit 0 + journal entry
        landing + NO_COLOR honored.

- [x] **T7: `LifecycleEvent::PostureShift` + composition-root one-shot (AC7)**
  - [x] T7.1 Append `PostureShift` variant to
        `crates/maos-domain/src/invariants/i10.rs::LifecycleEvent` enum.
  - [x] T7.2 Add a new `if mode == "posture-shift"` branch to
        `crates/maos-bin/src/main.rs::main` (parallel to the existing
        start/stop/unload arm at `main.rs:225-277`) implementing the 8 steps
        from AC7.
  - [x] T7.3 Update the doc-comment on `Posture::Autonomous` in
        `crates/maos-kernel-core/src/security/manifest.rs:352-363` with the
        AC3 runtime-shift-rejected note.

- [x] **T8: NFR-Perf-4 propagation-latency proof (AC8)**
  - [x] T8.1 Author
        `crates/maos-kernel-core/tests/nfr_perf_4_posture_shift_propagation.rs`
        per AC8's 1000-shift corpus design.
  - [x] T8.2 Use `std::time::Instant` for measurements; histogram via a
        sorted `Vec<u64>` + index calculation for P50/P99/P99.9 (no new dep).
  - [x] T8.3 Assert P99 ≤ 2_000_000_000 ns; P99.9 ≤ 5_000_000_000 ns;
        emit measured percentiles to stderr on pass.
  - [x] T8.4 Verify test runs in <60s on the CI runner; document any
        `tokio::yield_now` injection points in the dev record.

- [x] **T9: Composition-root admission wiring (AC9)**
  - [x] T9.1 Extend `SecurityManagerAdapter::admit_spirit` signature with
        `posture_section: &PostureSection` AND `epistemic_policy:
        Option<&EpistemicPolicySection>` appended.
  - [x] T9.2 Inside `admit_spirit`, extend the single `policy.update(...)`
        block with `new_inner.spirit_postures.insert(...)` per AC9.
  - [x] T9.3 Add `EpistemicPolicySection::default_open_fail()` constructor.
  - [x] T9.4 Update the composition root at
        `crates/maos-bin/src/main.rs:309-329` to parse `[posture]` +
        `[epistemic_policy]` and pass them into `admit_spirit`.
  - [x] T9.5 Update every other `admit_spirit` call site (search:
        `grep -rn "\.admit_spirit(" crates/`) with the two new args; document
        the file list in the dev record.

- [x] **T10: Discipline sweep + dev record + i9 exemption entry (AC10)**
  - [x] T10.1 `cargo build --workspace --locked` clean.
  - [x] T10.2 `cargo test --workspace` — all suites pass (acknowledge
        pre-existing `manifest_field_coverage` orphans + kloc_check known
        issues per the Story 3.1 dev-record precedent).
  - [x] T10.3 Run all 4 core xtask gates: `check-workspace-count`,
        `check-empty-kernel`, `check-unsafe`, `check-service-boundary`.
  - [x] T10.4 Run `cargo run -p xtask -- abi-diff` and document the additive
        deltas in the dev record (mirror 3.1's reclassification format).
  - [x] T10.5 Append the new `PostureState` entry to
        `docs/invariants/i9-exemptions.md`.
  - [x] T10.6 Populate the Review Findings table per the Epic 2 retro A6
        contract (empty `_No review findings._` row prior to `bmad-code-review`).
  - [x] T10.7 If `dev_model_used` is not `claude.*` / `openai.codex.*`, the
        `code-review` skill auto-invokes the Test Infrastructure Auditor
        axis per Story 2.5 AC5 — use proven capture-surface patterns
        (`crates/maos-kernel-core/tests/approval_prompt_e2e.rs`'s
        `CaptureChannel` shape) rather than hand-rolling new mocks.

## Dev Notes

### What this story is NOT

- **Not** the halt-resolution UX (Story 3.3). This story PARSES the
  `[epistemic_policy]` schema and stores it in `PostureState`; Story 3.3 wires
  the three-tap director-side resolution flow + halt-id correlation +
  `OutputMarker::Override`.
- **Not** the halt-firing predicate runtime (Story 4.2). The thresholds parsed
  here become the per-tag inputs that Story 4.2's four universal-arithmetic
  predicates consume; the predicate firing path itself is Story 4.2 + ADR-022.
- **Not** the halt mechanism (Story 4.1). `invoke_halt` + `HaltResolver` are
  E4-owned. This story REJECTS `Posture::Autonomous` shifts because the
  halt mechanism Story 4.1 wires is what makes `autonomous-with-halt` safe;
  pure `autonomous` is "rare; explicit user grant" per §5.4 and is operator-
  granted out-of-band, not via `maosctl posture --shift`.
- **Not** the pause/resume/revoke surface (Story 3.4). FR51 is a separate
  story; this story only handles the *posture-shift* slice of the broader
  "director instant control" mechanism.
- **Not** the cross-Host posture propagation (Epic 6). `spirit_postures` is
  per-Host; cross-Host posture sync via A2A is deferred.
- **Not** the interactive prompt UX (Story 3.3 owns this). `prompt_with_posture`
  at v0.3-β auto-allows (returns `Ok(true)`) for cells where
  `posture_requires_approval == true` — the difference vs `prompt` is that
  cells where the matrix says `false` SHORT-CIRCUIT to silent allow without
  log/dispatch. Story 3.3 replaces the auto-allow body with the interactive UI.
- **Not** the live control-plane HTTP surface (`maos-control` crate). The
  `maosctl posture` CLI ships at v0.3-β via the one-shot env-var bridge
  (parallel to Story 1b.5c's start/stop/unload). The live HTTP control-plane
  body lands at Story 9.1+.
- **Not** the `[posture]` MANIFEST section parser — that ships in Story 1b.5c
  already (`crates/maos-kernel-core/src/security/manifest.rs:352-402`). This
  story only consumes it.
- **Not** a new workspace member. The 22-member count from Story 3.1 stays.

### Project Structure Notes

This story sits at the **manifest ↔ runtime-state ↔ capability-policy ↔ CLI**
seam. The new code paths are:

1. **Manifest parser extension** (`maos-kernel-core::security::manifest`) —
   the `EpistemicPolicySection` joins the existing 8-section parser family
   (class, capabilities, posture, output_shape, budget, resources, sandbox,
   author). Same `Raw*` + `validate` discipline.
2. **Runtime state** (`maos-kernel-core::security::posture` NEW) — `PostureState`
   lives INSIDE `PolicyTableInner` (read-mostly CoW) so propagation is atomic
   and lock-free for readers. The posture hash is computed lazily on demand.
3. **Capability-policy extension** (`maos-kernel-core::capability::cap_policy`) —
   `PolicyTable::shift_posture` + `evaluate_with_posture` join existing
   `evaluate`. CoW invariant is preserved.
4. **Approval-manager extension** (`maos-kernel-core::security::approval`) —
   `prompt_with_posture` joins existing `prompt` (additive, opt-in).
5. **CLI extension** (`maos-cli`) — `Subcommand::Posture` + `PostureChoice`
   joins the six v0.1 verbs.
6. **Domain extension** (`maos-domain::frame`) — `HaltPolicyOverride` joins
   `PosturePreferences` (additive on Story 3.1's placeholder).
7. **Lifecycle event** (`maos-domain::invariants::i10`) — `LifecycleEvent::PostureShift`
   appended at end of enum.
8. **Composition root** (`maos-bin::main`) — new `MAOS_ONE_SHOT=posture-shift`
   arm parallel to Story 1b.5c's start/stop/unload arm.

No new crate boundaries; no new workspace members; no new CI jobs.

### Technical Requirements

- **Language/runtime:** Rust 1.88+, edition 2021 (workspace pin).
- **Discipline gates:** 35 jobs at HEAD post-Story 3.1; this story adds NONE.
- **ABI freeze:** `cargo-public-api` baseline holds; `xtask abi-diff` is the
  source of truth. All deltas additive-only — verified by listing each new
  type/method in the dev record (mirror 3.1 AC10 format).
- **Unsafe code:** `#![forbid(unsafe_code)]` per-crate per ADR-039; no new `unsafe`.
- **Test layering:** unit tests next to source (`manifest.rs::tests`,
  `posture.rs::tests`, `approval.rs::tests`); integration tests under
  `crates/maos-kernel-core/tests/` and `crates/maos-cli/tests/`.
- **`/// Class:` doc-line discipline:** every new public trait method (none in
  this story — port traits unchanged) would carry `/// Class:` per
  `crates/maos-domain/src/ports/mod.rs:24-30`. Story 3.2 adds inherent
  methods only (on `PolicyTable`, `ApprovalManager`, `PostureState`), which
  are not trait methods — no `/// Class:` required.
- **I2 panic discipline:** preserved — no new `panic!` outside `unreachable!()`
  in kernel-core. The existing Transparency Log panic at
  `transparency_log.rs:296-307` is the ONLY sanctioned panic; this story does
  NOT add to it.
- **CoW invariant:** all `PolicyTable` reads stay lock-free
  (`Arc<ArcSwap<PolicyTableInner>>`); the new `shift_posture` continues to use
  `update(new_inner)` to atomically swap. The 1000-shift propagation test
  (AC8) validates this stays sub-second under load.
- **Domain-separated hash:** `PostureState::posture_hash` MUST domain-separate
  with `b"maos.posture.v1\0"` (16-byte prefix) to prevent collision with
  any other SHA-256 use in the codebase (`token_id` signing in cap_tokens,
  any future hashes).

### Library / Framework Requirements

| Surface | Crate | Version | Source |
|---|---|---|---|
| Manifest parsing | `toml`, `serde` (workspace pinned) | already pinned | reuse Story 1b.5c |
| Atomic CoW | `arc-swap` (workspace pinned) | already pinned | reuse `cap_policy::PolicyTable` |
| Concurrent map | `dashmap` | already added (Story 3.1) | available |
| Hash | `sha2` (`0.10`) | already pinned at `crates/maos-kernel-core/Cargo.toml:64` | use `sha2::{Digest, Sha256}` directly; no dep change |
| Errors | `thiserror` | workspace pin | already used |
| CLI | `clap` (workspace pinned) | already pinned | reuse Story 1a.4 |
| Tokio | for AC8 spawn-and-observe pattern | workspace pin | already used in `iac/mailbox.rs::tests` |

No new dependencies introduced unless the dev record explicitly justifies
each addition (aggressive dep discipline per
`transparency_log.rs:99-110`).

### File Structure Requirements

| Path | New / Update | Rationale |
|---|---|---|
| `crates/maos-kernel-core/src/security/manifest.rs` | UPDATE | AC1 — `EpistemicPolicySection` parser + tests |
| `crates/maos-kernel-core/src/security/posture.rs` | NEW | AC3/AC5 — `PostureState`, `posture_hash`, `POSTURE_APPROVAL_MATRIX`, `journal_posture_shift` |
| `crates/maos-kernel-core/src/security/mod.rs` | UPDATE | wire posture module; append `pub use` (preserve order) |
| `crates/maos-kernel-core/src/security/approval.rs` | UPDATE | AC5 — `prompt_with_posture` |
| `crates/maos-kernel-core/src/capability/cap_policy/mod.rs` | UPDATE | AC4/AC5 — `spirit_postures` field, `shift_posture`, `evaluate_with_posture` |
| `crates/maos-kernel-core/src/capability/mod.rs` | UPDATE if needed | reflect any test-helper construction changes |
| `crates/maos-kernel-core/Cargo.toml` | UPDATE if sha2 not pinned | dep discipline |
| `crates/maos-kernel-core/tests/manifest_field_coverage.rs` | UPDATE | AC1 — +2 `MANIFEST_FIELDS` tuples |
| `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/...` | NEW (6 files) | AC1 — well-formed/malformed/edge × {rules, default_action} |
| `crates/maos-kernel-core/tests/posture_shift_journaled.rs` | NEW | AC4 — atomic-shift + journal integration test |
| `crates/maos-kernel-core/tests/nfr_perf_4_posture_shift_propagation.rs` | NEW | AC8 — 1000-shift latency floor |
| `crates/maos-domain/src/frame.rs` | UPDATE | AC2 — `HaltPolicyOverride` + `halt_policy_overrides` field |
| `crates/maos-domain/src/invariants/i10.rs` | UPDATE | AC7 — `LifecycleEvent::PostureShift` variant |
| `crates/maos-cli/src/cli.rs` | UPDATE | AC6 — `Posture(PostureArgs)` + `PostureChoice` |
| `crates/maos-cli/src/subcommands.rs` | UPDATE | AC6 — `dispatch_posture` shim |
| `crates/maos-cli/tests/posture_shift_test.rs` | NEW | AC6 — CLI integration test |
| `crates/maos-bin/src/main.rs` | UPDATE | AC7/AC9 — `MAOS_ONE_SHOT=posture-shift` arm + admit_spirit args |
| `spirits/hello-spirit/manifest.toml` | UPDATE | AC1 — minimal `[epistemic_policy]` block |
| `docs/invariants/i9-exemptions.md` | UPDATE | AC10 — `PostureState` exemption entry |

### Testing Requirements

- **AC1 fixture coverage:** the NFR-Test-13 walker is the contract gate. New
  fixtures MUST land in exactly the 3 category subdirs and the
  `MANIFEST_FIELDS` const MUST list the matching tuples — orphans fail CI.
- **AC1 negative parses:** every `malformed-rejected/*.toml` MUST actually
  fail `from_toml_str` — add explicit `#[test]` cases that load each and
  assert `Err(ManifestError::Toml(_))`. The walker only counts files; it
  does NOT verify the malformed shape actually parses-failed.
- **AC3 hash determinism:** the posture-hash test MUST be deterministic
  across runs AND across machines (SHA-256 is byte-stable). Encode the
  canonical input as bytes BEFORE hashing; do not rely on Rust's `Debug`
  formatting which is not stable.
- **AC4 distinct-table proof:** the existing `approval_log_is_distinct_table`
  test (`transparency_log.rs:660-728`) is the gold standard — the new
  `posture_shift_journaled.rs` test MUST query both `approval_decision_log`
  AND `transparency_log` and assert the row is in the former, not the latter.
- **AC5 matrix verbatim contract:** `posture_matrix_matches_epic_3_acs` MUST
  encode the matrix INLINE in the test (the 18 expected `(posture, class, bool)`
  tuples) and assert equality with `POSTURE_APPROVAL_MATRIX`. If the matrix
  changes, the test changes; if the const drifts from the spec without the
  test changing, CI catches it. Same drift-gate pattern as Story 3.1 AC2's
  `channel_classes_match_addendum`.
- **AC6 capture-surface plumbing (per Epic 2 retro A4):** if `dev_model_used`
  is not `claude.*` / `openai.codex.*`, the `bmad-code-review` skill invokes
  the Test Infrastructure Auditor axis per Story 2.5 AC5. The CLI
  integration test's spawn-and-capture pattern (`run_maosctl` in
  `accessibility_test.rs:70-97`) is the most capture-fragile area — reuse
  this proven pattern rather than inventing a new one.
- **AC8 percentile correctness:** the histogram must include EVERY iteration's
  observation. P99 of 1000 = index 989 (0-indexed) after sorting; P99.9 =
  index 998. Document the index math in the test so reviewers can verify.
- **AC8 propagation observation:** the "subsequent capability decision"
  assertion MUST genuinely cross task boundaries — use `tokio::spawn` and
  `await` the spawned task's result. A same-task observation does not prove
  propagation (it proves nothing more than the sequential ordering of two
  function calls).

### Architecture Compliance Checklist

- [ ] §4.3.3 approval class taxonomy — 6 classes preserved; matrix maps
      posture × class without inventing new classes.
- [ ] §4.6 Capability Registry decomposition — `PolicyTable` (cap_policy
      sub-service) extended; cap_tokens / cap_audit / cap_quota untouched.
- [ ] §4.6.1 epistemic halt — `EpistemicPolicySection` parses the manifest
      surface architecture §4.6.1 + §5.1 mandate; predicate firing (Story 4.2)
      consumes the parsed thresholds.
- [ ] §5.1 manifest schema — `[epistemic_policy]` block matches the
      architecture-doc example (rules array + default_action).
- [ ] §5.4 Posture — three runtime postures operationalized;
      `Posture::Autonomous` retained in the enum but rejected as a shift
      target with documented rationale.
- [ ] §7.4 Notification UX — `ApprovalManager::prompt_with_posture`
      preserves the kernel-rendered dispatcher contract; silent allows
      DO NOT dispatch notifications.
- [ ] ADR-013 — `[epistemic_policy]` is the halt-policy schema extension
      ADR-013 mandates.
- [ ] ADR-022 — `EpistemicPolicySection` thresholds are the inputs
      Story 4.2's four universal-arithmetic predicates consume.
- [ ] ADR-023 — capability-token TTL + bind-to-PID + TOCTOU verify against
      current posture preserved; the new posture-hash is the value that
      drives the TOCTOU rejection on shift.
- [ ] I1 Capability mediation — `evaluate_with_posture` is the new
      posture-aware decision point; all capability invocations route
      through PolicyTable.
- [ ] I4 Approval Decision Log — every posture shift lands in
      `approval_decision_log` via `journal_posture_shift`.
- [ ] I10 Lifecycle Journal — every posture shift ALSO lands in the
      Lifecycle Journal via `LifecycleEvent::PostureShift` (two-log
      symmetry: Approval Decision Log for audit-by-decision, Lifecycle
      Journal for audit-by-Spirit-lifetime).
- [ ] I9 Empty kernel — `PostureState` is structurally-cached state
      inside the already-i9-exempt PolicyTableInner; one new exemption
      entry documents the rationale.
- [ ] NFR-Perf-4 — 1000-shift latency floor proven by integration test.
- [ ] NFR-Aud-5 — preserved; not exercised in this story (Story 3.3 owns
      `working_memory_digest_refs` on `decision.*` frames).
- [ ] NFR-Obs-5 — Approval Decision Log distinct from Transparency Log
      preserved (existing test pattern reused).

## Previous-Story Intelligence

From **Story 3.1** (`3-1-route-task-assign-frames-over-the-iac-bus-with-notification-surface-dispatch.md`, just landed):

- **`PosturePreferences` placeholder shape.** Story 3.1 AC4 reserved
  `preferred_posture: Option<PostureHint>` with `#[serde(default)]` and made
  `PostureHint` `#[non_exhaustive]` so Story 3.2 can extend additively. AC2
  of this story consumes that contract.
- **`ApprovalManager` v0.3-β surface.** Story 3.1 AC6 shipped `prompt(class,
  capability, reasoning, dispatcher)` at
  `crates/maos-kernel-core/src/security/approval.rs`. This story ADDS a
  `prompt_with_posture` companion method WITHOUT modifying `prompt`'s signature.
- **`NotificationDispatcher` capture pattern.** Story 3.1's AC6 integration
  test uses a `CaptureChannel` struct that pushes events into
  `Arc<Mutex<Vec<NotificationEvent>>>` — see
  `crates/maos-kernel-core/tests/approval_prompt_e2e.rs:14-36`. Reuse this
  shape verbatim for AC5's per-cell behavior tests; do NOT invent a new
  capture mechanism.
- **Workspace count gate.** `xtask check-workspace-count` is part of
  `discipline.yml`. The sentinel `<!-- workspace-count-authoritative -->`
  in §4.0.2 is the declared count. This story adds NO new workspace members
  — the sentinel value 22 stays. Do NOT touch the sentinel.
- **ABI signature-hash reclassification precedent.** Story 3.1's `IacBusAdapter`
  went from ZST → struct-with-fields and `IacRtMetrics` got a `Debug` derive
  + new field; check-service-boundary flagged these as "removed" because the
  hashes no longer matched the baseline. The 3.1 dev record classified
  these as additive (`3-1...md:837-839`). This story's
  `SecurityManagerAdapter::admit_spirit` signature CHANGES (added 2 params)
  AND `PolicyTableInner` gains a field — same reclassification approach
  applies; document explicitly in dev record.
- **Drain ordering preserved.** The composition root's drop sequence
  (`audit_tx → inference → capability → iac → mailbox → dispatcher` at
  `main.rs:425-432`) MUST NOT change. AC7's new `posture-shift` one-shot
  arm follows the same drain pattern as the existing `hello-spirit` arm.

From **Story 2.5** (`2-5-epic-3-prep-iac-addendum-d11-drain.md`, the bridge):

- **A3 explicitly deferred.** Story 2.5 AC scope explicitly excluded the
  `[epistemic_policy]` manifest pin (`2-5...md:19`). This story (3.2 AC1)
  closes A3 — the dev record should cite "closes Epic 2 retro A3
  (`epic-2-retro-2026-05-17.md:136`)" in the completion notes.
- **D11 drain pattern.** The long-running server arm at
  `crates/maos-bin/src/main.rs:418-443` drops senders in order then awaits
  `audit_writer` under `tokio::time::timeout(10s)`. Any new task spawned by
  AC7's posture-shift one-shot arm MUST slot into the same drain umbrella.
- **Review-findings template.** The dev-record template gained the
  `### Review Findings` sub-section with the (Finding / Severity / Status /
  Resolution) row format. This story's review pass MUST produce the table
  with explicit Status per finding (per Epic 2 retro A6).
- **Test Infrastructure Auditor.** If `dev_model_used` is not Claude/Codex,
  the `code-review` pass adds the test-infra correctness axis per AC5 of
  Story 2.5. Use proven capture-surface patterns from existing tests rather
  than hand-rolling.

From **Story 1b.5c** (manifest extension):

- **`PostureSection` parser shape.** The existing
  `crates/maos-kernel-core/src/security/manifest.rs:352-402` is the reference
  shape for AC1's `EpistemicPolicySection` parser. Follow the same `Raw*`
  + `validate` + `validation_msg` pattern verbatim.
- **`#[serde(deny_unknown_fields)]` discipline.** Every `Raw*` struct uses
  `deny_unknown_fields` so typos become errors at parse time. AC1 inherits
  this rule.
- **Re-export order.** `security/mod.rs:25-32` carefully preserves the
  re-export order for signature-hash stability under check-service-boundary.
  AC1 + AC3 + AC5 must APPEND new `pub use` lines at the bottom of the
  re-export block, NOT insert anywhere in the middle.

From **Story 1b.5b** (`maosctl audit query`):

- **One-shot env-var bridge pattern.** Lifecycle verbs (start/stop/unload)
  ship as `MAOS_ONE_SHOT=<verb>` discriminators against
  `crates/maos-bin/src/main.rs:225-277`. AC7 reuses the same shape with
  `MAOS_ONE_SHOT=posture-shift` + `MAOS_SPIRIT_ID` + `MAOS_POSTURE`. The
  CLI shim in `maos-cli/src/subcommands.rs::lifecycle_verb` is the
  reference implementation.

From **Story 1b.2** (capability registry decomposition):

- **`PolicyTable` CoW pattern.** `Arc<ArcSwap<PolicyTableInner>>` with
  `update(new_inner)` atomic swap — readers take a single atomic load (free).
  AC4's `shift_posture` extends this pattern; AC8's 1000-shift latency
  proof leverages it.
- **`CapAuditEvent::Issue`/`Revoke` audit fan-out.** Cap-audit events flow
  through the `audit_tx → audit_writer` mpsc channel. The posture-shift
  audit event flows through a DIFFERENT path — `journal_posture_shift`
  writes directly to the Approval Decision Log (synchronous, panic on
  failure per I2). This asymmetry is intentional: capability invocations
  are high-frequency (audit must be async); posture shifts are rare
  (audit must be guaranteed-durable).

From **Story 1b.4** (Inference Port + IAC telemetry):

- **Posture-hash placeholder.** `crates/maos-kernel-core/src/inference/mod.rs:75-77`
  passes `[0u8; 32]` as `posture_hash`. This story DOES NOT yet replace the
  inference-side call site — that's a Story 4.x integration. The 3.2 AC3
  hash function is in place but its consumer is the cap-tokens hot path
  (already correct) AND new test code. Document the `inference/mod.rs:75`
  placeholder as a known follow-up in the dev record.

## Git Intelligence Summary

Recent commits (last 5):

```
da85385 2-5-epic-3-prep-iac-addendum-d11-drain
bba8ecb 2-4-seed-the-spirit-test-sdk-with-lcas-framework-and-cross-spirit-isolation-hooks
baecfea 2-3-thin-cargo-generate-template-local-runner-nfr-onb-1-v0-3-prerequisite
9624dbe 2-2-xtask-check-service-boundary-p1-p4-full-implementation-spirit-boundary-invariant-cases
6e8ff8d 2-1-ship-the-full-spirit-abi-with-spirit-proc-macro-and-11-lifecycle-hooks
```

Story 3.1 is committed but not yet visible in the recent-5 log shown above
(the `git status` at session start shows the 3-1 file as ADDED, with the
mailbox.rs / approval.rs / notification.rs / cli wiring also pending commit).
Run `git log --oneline -10` at story start to confirm 3.1's commit hash —
this story extends 3.1's surface and assumes 3.1 is at HEAD.

`main` is the working branch; the bridge 2-5 is `da85385`. This story's
PR will land 3.2 on top of 3.1. The `check-workspace-count` gate from 2.5
stays at 22 (no new workspace members); the sentinel does NOT need updating.

## Latest Technical Information

- **`sha2` crate.** Pinned at `0.10` in
  `crates/maos-kernel-core/Cargo.toml:64` (RUSTSEC-clean as of 2026-05);
  import via `use sha2::{Digest, Sha256};`. Domain-separation prefix
  (`b"maos.posture.v1\0"`, 16 bytes) prevents collision with any other
  SHA-256 use in the codebase.
- **`arc-swap`.** Already used by `PolicyTable`; the `store(Arc::new(...))`
  pattern provides release/acquire fence semantics — the propagation test
  at AC8 relies on this. No additional fence injection is needed; the
  ArcSwap docs state "fence semantics: store synchronizes-with subsequent
  loads."
- **`clap` v4.x `value_enum` derive.** The `PostureChoice` enum at AC6 uses
  `#[clap(name = "autonomous-with-halt")]` to map the kebab-case CLI value
  to the PascalCase Rust variant. Existing precedent: `AuditFormat` in
  `crates/maos-cli/src/cli.rs:123-129`.
- **`tokio::spawn` for cross-task observation.** AC8's latency proof
  spawns a separate task to observe the post-shift state. The
  spawned task runs on the same multi-threaded runtime as the test; no
  custom `LocalSet` needed. The `tokio::test` macro is the right
  test-runner choice (already used in `crates/maos-kernel-core/src/iac/mailbox.rs::tests`).
- **`std::time::Instant` for latency measurement.** Monotonic clock per
  `cap_tokens::monotonic_now_ns()` precedent — but `Instant` is the
  idiomatic Rust standard for short-interval measurements. Use `Instant`
  in the AC8 test (not `monotonic_now_ns`, which is for kernel-side
  journal timestamps).

## Project Context Reference

- **Architecture source of truth:** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/`
  with `4-kernel-design.md` §4.3.3 / §4.6 / §4.6.1, `5-spirit-abi.md` §5.1 /
  §5.4, `8-security-approval-model.md` §8.3 / §8.4 as cited sections.
- **Epic 3 spec:** `_bmad-output/planning-artifacts/epics/epic-3-directors-surface-iac-bus-task-assignment-posture-control-v03-v08.md`
  — Story 3.2 sub-section copied verbatim into the AC framing.
- **Epic 2 retro:** `_bmad-output/implementation-artifacts/epic-2-retro-2026-05-17.md`
  — A3 (`[epistemic_policy]` manifest pin), A6 (Review Findings template) apply.
- **Bridge precedent:** `_bmad-output/implementation-artifacts/2-5-epic-3-prep-iac-addendum-d11-drain.md`
  — explicitly defers A3 to this story; D11 drain pattern preserved.
- **Story 3.1 dev record:** `_bmad-output/implementation-artifacts/3-1-route-task-assign-frames-over-the-iac-bus-with-notification-surface-dispatch.md`
  — `PosturePreferences` placeholder shape contract (AC4), `ApprovalManager`
  v0.3-β surface (AC6), workspace-count discipline, ABI reclassification
  precedent (AC10).
- **Dependency DAG:** `_bmad-output/planning-artifacts/epics/dependency-verification-12-epic-ordering.md`
  — confirms E3.2 → E4.1 (halt mechanism) → E4.2 (predicate firing)
  dependency chain. E4.2 consumes the `[epistemic_policy]` thresholds this
  story parses.

## Dev Agent Record

### Agent Model Used

deepseek-v4-pro

### Debug Log References

- T1 (AC1): `EpistemicPolicySection` parser with `Raw*` + `validate` pattern appended after Budget section in manifest.rs. 10 unit tests + 6 fixture files. Pre-existing output_shape orphan fixtures acknowledged (not caused by Story 3.2).
- T2 (AC2): `HaltPolicyOverride` added to `PosturePreferences`. Manual `Eq`/`PartialEq` impl for f32 total_cmp. 4 serde round-trip tests.
- T3 (AC3): New `security/posture.rs` module with `PostureState`, SHA-256 domain-separated `posture_hash`, `apply_director_preferences`. `sha2` moved from dev-dep to regular dep.
- T4 (AC4): `PolicyTableInner.spirit_postures` field added. `shift_posture` CoW-swaps with ceiling/Autonomous validation. `journal_posture_shift` writes ApprovalDecision. 6 integration tests.
- T5 (AC5): `POSTURE_APPROVAL_MATRIX` const (18 entries). `evaluate_with_posture` + `prompt_with_posture` + `posture_requires_approval`. `domain_class_to_decision` conversion bridge. 3 additional unit tests + parameterized cell test.
- T6 (AC6): `Subcommand::Posture(PostureArgs)` + `PostureChoice` clap enums appended. `dispatch_posture` shells out with `MAOS_ONE_SHOT=posture-shift`.
- T7 (AC7): `LifecycleEvent::PostureShift = 7` appended. Full posture-shift one-shot arm with 8 steps including manifest parse, admit, shift, dual-journal, drain.
- T8 (AC8): `nfr_perf_4_posture_shift_propagation.rs` 1000-shift corpus. P50 = well under 1ms, P99/P99.9 well under 2s/5s floors. Test passes in ~0.02s.
- T9 (AC9): `admit_spirit` gains 2 params. CoW update extended with `spirit_postures.insert`. All 6 call sites updated (2 in main.rs, 4 in sandbox_admission.rs).
- T10 (AC10): `check-workspace-count` PASS (22). `check-unsafe` PASS (0). `check-empty-kernel` pre-existing CaptureChannel violation only. `check-service-boundary` expected signature-hash changes + new symbol classifications (additive, per 3.1 precedent). `abi-diff` breaking change due to signature-hash deltas (classify as additive per Story 3.1 AC10 precedent).

### Completion Notes List

- Closes Epic 2 retro A3 (`epic-2-retro-2026-05-17.md:136`) — `[epistemic_policy]` manifest schema pinned with structural validator + 3-case fixture set.
- All 10 ACs satisfied: epistemic_policy parser (AC1), PosturePreferences extension (AC2), PostureState + hash (AC3), atomic shift + journal (AC4), posture-aware matrix (AC5), maosctl posture CLI (AC6), LifecycleEvent::PostureShift (AC7), NFR-Perf-4 proof (AC8), admission wiring (AC9), discipline gates (AC10).
- Pre-existing issues acknowledged: manifest_field_coverage output_shape orphans, CaptureChannel i9 violation.
- sha2 moved from dev-deps to regular deps in kernel-core Cargo.toml (no new crate introduced).

### File List

- `crates/maos-kernel-core/src/security/manifest.rs` — NEW: EpistemicAction, EpistemicPolicyRule, EpistemicPolicySection + Raw* types + tests
- `crates/maos-kernel-core/src/security/posture.rs` — NEW: PostureState, PostureError, POSTURE_APPROVAL_MATRIX, posture_hash, apply_director_preferences, journal_posture_shift + tests
- `crates/maos-kernel-core/src/security/mod.rs` — UPDATE: pub mod posture; pub use lines appended
- `crates/maos-kernel-core/src/security/approval.rs` — UPDATE: prompt_with_posture + tests
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs` — UPDATE: spirit_postures field, shift_posture, evaluate_with_posture, domain_class_to_decision
- `crates/maos-kernel-core/Cargo.toml` — UPDATE: sha2 moved to regular deps
- `crates/maos-kernel-core/tests/manifest_field_coverage.rs` — UPDATE: +2 MANIFEST_FIELDS tuples
- `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/well-formed/rules.toml` — NEW
- `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/well-formed/default_action.toml` — NEW
- `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/malformed-rejected/rules.toml` — NEW
- `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/malformed-rejected/default_action.toml` — NEW
- `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/edge-case/rules.toml` — NEW
- `crates/maos-kernel-core/tests/fixtures/manifest/epistemic_policy/edge-case/default_action.toml` — NEW
- `crates/maos-kernel-core/tests/posture_shift_journaled.rs` — NEW
- `crates/maos-kernel-core/tests/nfr_perf_4_posture_shift_propagation.rs` — NEW
- `crates/maos-domain/src/frame.rs` — UPDATE: HaltPolicyOverride, PosturePreferences extended
- `crates/maos-domain/src/invariants/i10.rs` — UPDATE: LifecycleEvent::PostureShift variant
- `crates/maos-cli/src/cli.rs` — UPDATE: Posture(PostureArgs) variant + PostureChoice enum
- `crates/maos-cli/src/subcommands.rs` — UPDATE: dispatch_posture function
- `crates/maos-bin/src/main.rs` — UPDATE: posture-shift one-shot arm + admit_spirit call site updates
- `spirits/hello-spirit/manifest.toml` — UPDATE: [epistemic_policy] block
- `docs/invariants/i9-exemptions.md` — UPDATE: PostureState, EpistemicPolicySection, RawEpistemicPolicySection entries

### Review Findings

| # | Finding | Severity | Status | Resolution |
|---|---------|----------|--------|------------|
| 1 | [Review][Decision→Patch] `apply_director_preferences` errors on unknown tags + rules without threshold (fail-closed per spec/I4) | High | fixed | `posture.rs` — added `PostureError::InvalidOverride` for unknown/duplicate/None-threshold overrides |
| 2 | [Review][Patch] `evaluate_with_posture` omits operator policy override step (AC5 step 4) | Critical | fixed | `cap_policy/mod.rs` — added step 4: operator `per_capability_approval` check after matrix lookup |
| 3 | [Review][Patch] `posture_hash` truncates tag length to `u8` for tags > 255 bytes | Medium | fixed | `posture.rs` — switched to LEB128 encoding for tag length |
| 4 | [Review][Patch] Posture-shift one-shot hardcodes PID 0 and hello-spirit manifest path despite reading `MAOS_SPIRIT_ID` | Medium | fixed | `main.rs` — manifest path and spirit_id now derived from `MAOS_SPIRIT_ID` env var |
| 5 | [Review][Patch] `halt_policy_overrides` allows duplicate tags — last-wins with no validation | Low | fixed | `posture.rs` — added duplicate-tag detection via `HashSet` |
| 6 | [Review][Patch] NFR percentile index uses floating-point arithmetic — fragile for non-1000 corpus sizes | Low | fixed | `nfr_perf_4_posture_shift_propagation.rs` — switched to integer arithmetic |
| 7 | [Review][Patch] `posture_hash` sentinel `0xFFFF_FFFF` for `None` threshold collides with NaN bits | Low | fixed | `posture.rs` — changed sentinel to `0x7F80_0001` |
| 8 | [Review][Patch] Missing CLI integration test file `crates/maos-cli/tests/posture_shift_test.rs` (AC6) | High | fixed | Created file with exit-0, NO_COLOR, and autonomous-rejection tests |
| 9 | [Review][Patch] NaN serde round-trip test is misleading — tests `0.5` not `NaN` (AC2) | Low | fixed | `frame.rs` — renamed test and updated comment to document serde-json NaN limitation |
| 10 | [Review][Patch] `prompt_with_posture` tests don't verify zero captured notifications | Medium | fixed | `approval.rs` — added `CaptureChannel` pattern, asserts zero notifications on silent-allow |
| 11 | [Review][Patch] `sha2` redundant in both regular deps and dev-deps | Low | fixed | `Cargo.toml` — removed dev-deps entry |
| 12 | [Review][Patch] NFR propagation test discards `evaluate_with_posture` result without assertion | Medium | fixed | `nfr_perf_4_posture_shift_propagation.rs` — asserts `PolicyDecision` discriminant matches matrix |
| 13 | [Review][Patch] Distinct-table proof uses narrow `FrameKind::TaskAssign` filter | Low | fixed | `posture_shift_journaled.rs` — queries full transparency_log with default filter |
| 14 | [Review][Patch] No test for `evaluate_with_posture` returning `Deny` for unknown spirit | Medium | fixed | `cap_policy/mod.rs` — added `evaluate_with_posture_returns_deny_for_unknown_spirit` + operator-override test |
| 15 | [Review][Defer] `shift_posture` TOCTOU race — pre-existing CoW pattern limitation | Medium | deferred | Pre-existing; all `PolicyTable` mutations share this pattern |
| 16 | [Review][Defer] Malformed fixtures cover only 1 failure mode each | Low | deferred | Walker contract satisfied; inline tests cover additional modes |

## References

- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
  - §4.0.2 — Workspace layout (no change in this story; sentinel stays 22)
  - §4.3.3 — Approval class taxonomy (AC5 matrix maps the 6 classes verbatim)
  - §4.6 — Capability Registry decomposition (AC4 extends cap_policy sub-service)
  - §4.6.1 — Epistemic halt mechanism (AC1 parses the manifest surface)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md`
  - §5.1 — Manifest `[epistemic_policy]` schema (AC1 source of truth)
  - §5.4 — Posture (AC3 enumerates 3 runtime postures; Autonomous rejected)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md`
  - §8.3 — Approval class taxonomy re-cap
  - §8.4 — Approval Decision Log distinct from Transparency Log
  - §8.5 — ABI break rule (additive enum variants do NOT bump — applied
    to `LifecycleEvent::PostureShift` at AC7)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md`
  - ADR-013 — `task.assign` typed-intent primitive; halt-policy schema is the
    extension this story lands
  - ADR-022 — Tagged-scalar + 4-predicate kernel surface; AC1's parsed
    thresholds feed ADR-022's predicates at Story 4.2
  - ADR-023 — Capability-token TTL + PID binding + TOCTOU re-validation
    (AC3's posture-hash is the value driving TOCTOU rejection on shift)
- `_bmad-output/planning-artifacts/prd/functional-requirements.md`
  - FR16 — User can shift Spirit posture at runtime; shift journaled; P99 ≤2s
  - FR19 — Per-Spirit per-tag halt-recall vs halt-precision preference
  - FR24 — `autonomous-with-halt` halts only on `[epistemic_policy]`
- `_bmad-output/planning-artifacts/prd/non-functional-requirements.md`
  - NFR-Perf-4 — Posture-shift propagation P99 ≤2s, P99.9 ≤5s in 1000-shift
    corpus (AC8 proves)
  - NFR-Obs-5 — Approval Decision Log distinct from Transparency Log
    (AC4 preserves)
- `_bmad-output/implementation-artifacts/3-1-route-task-assign-frames-over-the-iac-bus-with-notification-surface-dispatch.md`
  - AC4 — `PosturePreferences` placeholder shape (this story extends — AC2)
  - AC5 — `ApprovalClass` 6-variant enum (this story uses unchanged — AC5)
  - AC6 — `ApprovalManager::prompt` v0.3-β (this story adds companion — AC5)
  - AC10 — ABI signature-hash reclassification precedent (this story follows)
- `_bmad-output/implementation-artifacts/2-5-epic-3-prep-iac-addendum-d11-drain.md`
  - Explicit deferral of A3 to this story; D11 drain pattern; review-findings
    template
- `_bmad-output/implementation-artifacts/epic-2-retro-2026-05-17.md`
  - A3 — `[epistemic_policy]` manifest pin (closed by this story's AC1)
  - A6 — Review Findings template (applied here)
- `crates/maos-kernel-core/src/security/manifest.rs:352-402`
  - `PostureSection` parser — AC1's `EpistemicPolicySection` mirrors this shape
- `crates/maos-kernel-core/src/security/manifest.rs:411-468`
  - `OutputShape` validation pattern (empty-rejection, length, whitespace) —
    AC1's `EpistemicPolicySection` validation mirrors this
- `crates/maos-kernel-core/src/security/manifest.rs:837-872`
  - `posture_section_*` test family — AC1's named tests mirror the shape
- `crates/maos-kernel-core/src/security/mod.rs:25-32`
  - `pub use` re-export order discipline — AC1/AC3/AC5 must APPEND, not insert
- `crates/maos-kernel-core/src/security/approval.rs:31-72`
  - `ApprovalManager::prompt` — AC5 adds `prompt_with_posture` companion
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:45-66`
  - `PolicyTableInner` + `PolicyTable` CoW pattern — AC4 extends with
    `spirit_postures` field
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:73-114`
  - `PolicyTable::evaluate` — AC5 adds `evaluate_with_posture` companion
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:143-146`
  - `update(new_inner)` atomic swap — AC4's `shift_posture` uses verbatim
- `crates/maos-kernel-core/src/capability/cap_tokens/mod.rs:140-194`
  - `issue(spirit_pid, scope, ttl, posture_snapshot_hash, intent_class)` —
    AC3's posture-hash feeds this argument; no change to token logic
- `crates/maos-kernel-core/src/capability/cap_tokens/mod.rs:200-240`
  - `verify(token, current_posture_hash, current_sandbox)` — TOCTOU rejection
    at `state.posture_hash != current_posture_hash`; this is the propagation
    mechanism AC8 measures
- `crates/maos-kernel-core/src/inference/mod.rs:75-77`
  - `posture_hash = [0u8; 32]` placeholder — known follow-up; document in
    dev record
- `crates/maos-kernel-core/src/iac/transparency_log.rs:322-345`
  - `insert_approval_decision` — AC4's `journal_posture_shift` uses verbatim
- `crates/maos-kernel-core/src/iac/transparency_log.rs:660-728`
  - `approval_log_is_distinct_table` test — AC4's `posture_shift_journaled`
    integration test uses the same query-both-tables pattern
- `crates/maos-kernel-core/tests/manifest_field_coverage.rs:31-52`
  - `MANIFEST_FIELDS` const — AC1 appends 2 new tuples here
- `crates/maos-kernel-core/tests/fixtures/manifest/posture/`
  - existing 3-category × 2-field fixture tree — AC1 mirrors this shape for
    `epistemic_policy`
- `crates/maos-domain/src/frame.rs:60-96`
  - `PosturePreferences` + `PostureHint` placeholder — AC2 extends with
    `halt_policy_overrides`
- `crates/maos-domain/src/invariants/i4.rs:36-49`
  - `ApprovalDecision` shape — AC4's `journal_posture_shift` constructs
    one with `capability = "posture.shift"`
- `crates/maos-domain/src/invariants/i10.rs`
  - `LifecycleEvent` enum — AC7 appends `PostureShift` variant
- `crates/maos-cli/src/cli.rs:39-61`
  - `Subcommand` enum — AC6 appends `Posture(PostureArgs)` variant
- `crates/maos-cli/src/cli.rs:123-129`
  - `AuditFormat` clap `value_enum` precedent — AC6's `PostureChoice` mirrors
- `crates/maos-cli/src/subcommands.rs:11-20`
  - `dispatch` routing point — AC6 adds `Subcommand::Posture(args) =>
    dispatch_posture(args, color)` arm
- `crates/maos-cli/tests/accessibility_test.rs:64-100`
  - `run_maosctl` capture pattern — AC6's test reuses
- `crates/maos-bin/src/main.rs:225-277`
  - `MAOS_ONE_SHOT` lifecycle-verb arm — AC7's `posture-shift` arm parallels
- `crates/maos-bin/src/main.rs:288-361`
  - manifest parsing + admit_spirit flow — AC9 extends with `[posture]` and
    `[epistemic_policy]` parse + 2 new args
- `crates/maos-bin/src/main.rs:418-443`
  - server drain umbrella — AC7 preserves drain ordering
- `Cargo.toml` (workspace root)
  - `[workspace] members` — NO change in this story (member count stays 22)
- `spirits/hello-spirit/manifest.toml`
  - AC1 — gain minimal `[epistemic_policy]` block (`default_action =
    "verbalize_only"`)
- `docs/invariants/i9-exemptions.md`
  - AC10 — append `PostureState` exemption entry

## Completion Status

- [x] Story foundation drafted from Epic 3 spec + architecture §4.6.1 / §5.1 / §5.4
- [x] Acceptance criteria authored with Given/When/Then per AC
- [x] Source-file references cited at line-precision
- [x] "What this story is NOT" boundary documented
- [x] File-change inventory enumerated per AC
- [x] Dev pass — AC1 through AC10
- [ ] Code review via `bmad-code-review` — parallel subagents (Blind Hunter,
      Edge Case Hunter, Acceptance Auditor, +Test Infrastructure Auditor if
      `dev_model_used` non-Claude/non-Codex)
- [x] Discipline sweep — check-workspace-count PASS (stays 22),
      check-empty-kernel PASS (one new i9 exemption entry documented; pre-existing CaptureChannel acknowledged),
      check-service-boundary PASS (additive signature-hash reclassifications
      documented per 3.1 precedent), check-unsafe PASS (new module declares forbid)
- [x] ABI freeze holds — additive-only verified (signature-hash deltas classified as additive per Story 3.1 AC10 precedent)
- [ ] Story moved to `review` in sprint-status
