//! Story 11.1b — Cross-Form Equivalence Gate (tiered behavioral oracle).
//!
//! This module is the CORE harness that the WASM-vs-native equivalence story
//! feeds into. A "form" is the substrate a Spirit runs on: a native twin
//! process (`equiv-native-twin`) or a real wasmtime component
//! (`maos-wasm-runner` + `<fixture>.wasm`). Both forms are driven by the SAME
//! shared `equiv-fixture-logic` crate, so any behavioral divergence between
//! them is attributable to the FORM (the bridge / runner / sandbox), not the
//! Spirit logic.
//!
//! # Tiered comparison
//!
//! Captured effects are compared across two tiers:
//!
//! - **Invariant tier** — fields whose cross-form equivalence is the WHOLE
//!   POINT of the oracle (normalized frame sequence, halt reason, denial
//!   kind, region identity, audit shape). Any single divergence here is RED.
//!   Required match: **100 %**.
//! - **Cosmetic tier** — fields that may legitimately drift between forms
//!   (the logical `spirit_pid`, which both forms report for the same logical
//!   Spirit but which is not a behavioral invariant). Required match: **≥ 75 %**.
//!
//! The cosmetic threshold is intentionally loose so that harmless per-form
//! variation never flips the gate, while the invariant threshold is absolute
//! so that a real behavioral divergence always does. The cosmetic tier is
//! exercised by [`cosmetic_threshold_bites_through_invariant_green`] — it is
//! NOT vacuous: a corpus with enough `spirit_pid` drift drops cosmetic below
//! 75 % and flips the gate RED while the invariant tier stays 100 %.
//!
//! # F3 exclusion (tier-map honesty)
//!
//! Four domain fields are NOT carried by the `maos:spirit@1.0` WIT projection
//! and therefore cannot be compared cross-form: `intent`, `consent_envelope`,
//! `intent_lineage`, and `scope`. They are enumerated in
//! [`F3_EXCLUDED_FIELDS`] so the set is explicit, grep-able, and guarded by
//! mutation tests ([`f3_allowlist_is_pinned_to_the_dropped_set`],
//! [`normalize_ignores_exactly_the_f3_excluded_fields`], and
//! [`all_invariant_fields_are_guarded_against_demotion`]). Adding an invariant
//! field to the excluded set would silently turn a real divergence GREEN —
//! those tests prove that does not happen for every preserved field, not just
//! `logical_clock`.
//!
//! # Form-identity reflex (Task 6)
//!
//! The form identity on a capture is derived from the LIVE emitting process
//! (which subprocess produced it), never from a caller-applied label. There is
//! NO public capture helper that takes a free `SpiritForm` argument: a capture
//! is stamped either [`capture_native`] (the `equiv-native-twin` subprocess)
//! or [`capture_wasm`] (the `maos-wasm-runner` subprocess) — the function you
//! call IS the live-process identity. A stream that turns out to mix forms
//! internally (corruption) is rejected as a fail-loud RED
//! ([`mixed_form_stream_rejected`]), and a pair of streams that share a form
//! is a self-comparison that trivially agrees — the gate REJECTS it
//! ([`same_form_pair_rejected`]).
//!
//! # Anti-canned tripwire
//!
//! Under the `equiv-fault-inject` feature a helper perturbs a captured
//! invariant-tier effect AFTER capture but BEFORE comparison, proving the
//! comparator actually responds to the data rather than rubber-stamping GREEN.
//! The injection covers EVERY invariant class (not just frame-sequence), uses
//! the SAME saturating arithmetic as the real divergent transform, and is
//! proven to move the number even at the `u64::MAX` boundary
//! ([`fault_injection_covers_every_invariant_class`],
//! [`fault_injection_boundary_moves_under_distinct_perturbation`]).

use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::halt::{HaltId, HaltReceipt};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::capability::CapError;
use maos_domain::region::Region;
use maos_spirit_abi::identity::FrameKind;
use maos_wasm_host::codec;

use smallvec::SmallVec;

// ════════════════════════════════════════════════════════════════════════
// §1  Source-form identity + observed-effect types
// ════════════════════════════════════════════════════════════════════════

/// Source form identity — MUST be derived from the live emitting process, not
/// a caller-applied label. `Native` = the `equiv-native-twin` subprocess;
/// `Wasm` = the `maos-wasm-runner` subprocess driving a `.wasm` component.
///
/// There is no public helper that accepts a `SpiritForm` as a free argument:
/// captures are stamped via [`capture_native`] / [`capture_wasm`], so the form
/// is bound to whichever subprocess the caller actually drove.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpiritForm {
    Native,
    Wasm,
}

/// Invariant class of an observed effect — determines which [`EffectData`]
/// shape to expect and how to pair native/wasm captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvariantClass {
    FrameSequence,
    Halt,
    CapabilityDenial,
    RegionPin,
    Audit,
}

/// An observed effect from running a scenario through one Spirit form.
#[derive(Debug, Clone)]
pub struct CapturedEffect {
    pub form: SpiritForm,
    pub scenario: String,
    pub invariant_class: InvariantClass,
    pub data: EffectData,
}

/// The actual observed data for comparison. Each variant carries an
/// invariant-bearing projection (compared at the 100 % tier) and, where
/// relevant, a cosmetic projection (`spirit_pid`, compared at the ≥ 75 % tier).
///
/// Every variant is exercised by a live test — none is a dead declaration:
/// - `FrameSequence` — the three subprocess pipeline tests +
///   [`fault_injection_covers_every_invariant_class`].
/// - `HaltReceipt` / `CapabilityDenial` / `RegionViolation` / `AuditEntry` —
///   built from REAL domain types (`HaltReceipt`, `CapError`, `Region`, and
///   the real kernel audit kind strings) by the per-class coverage tests
///   ([`halt_effect_observed_and_compared`] et al.).
#[derive(Debug, Clone, PartialEq)]
pub enum EffectData {
    /// An ordered sequence of frames emitted by the form. Pure-invariant:
    /// the whole normalized sequence must match.
    FrameSequence(Vec<NormalizedFrame>),
    /// An epistemic-halt receipt. `halt_id` is invariant; `spirit_pid` is
    /// cosmetic (the logical Spirit identity, stable across forms for the
    /// same Spirit, but not a behavioral invariant).
    HaltReceipt { halt_id: String, spirit_pid: u32 },
    /// A capability-denial outcome. `error_kind` is invariant; `spirit_pid`
    /// is cosmetic.
    CapabilityDenial { error_kind: String, spirit_pid: u32 },
    /// A region-pin violation (I6). Both fields are invariant.
    RegionViolation {
        attempted_region: String,
        home_region: String,
    },
    /// An audit-log entry. `kind` and `count` are both invariant.
    AuditEntry { kind: String, count: usize },
}

impl EffectData {
    /// Stable tag for the active variant — used to detect a structural
    /// (cross-variant) divergence, which is itself an invariant divergence.
    fn discriminant(&self) -> &'static str {
        match self {
            EffectData::FrameSequence(_) => "frame_sequence",
            EffectData::HaltReceipt { .. } => "halt_receipt",
            EffectData::CapabilityDenial { .. } => "capability_denial",
            EffectData::RegionViolation { .. } => "region_violation",
            EffectData::AuditEntry { .. } => "audit_entry",
        }
    }

    /// Invariant-tier projection: the fields whose cross-form equivalence IS
    /// the oracle. A structural (discriminant) mismatch is reported here too,
    /// so a form that emits a `HaltReceipt` where the other emits a
    /// `FrameSequence` is RED, not silently dropped.
    fn invariant_matches(&self, other: &EffectData) -> bool {
        if self.discriminant() != other.discriminant() {
            return false;
        }
        match (self, other) {
            (EffectData::FrameSequence(a), EffectData::FrameSequence(b)) => a == b,
            (
                EffectData::HaltReceipt { halt_id: a, .. },
                EffectData::HaltReceipt { halt_id: b, .. },
            ) => a == b,
            (
                EffectData::CapabilityDenial { error_kind: a, .. },
                EffectData::CapabilityDenial { error_kind: b, .. },
            ) => a == b,
            (
                EffectData::RegionViolation {
                    attempted_region: a,
                    home_region: c,
                },
                EffectData::RegionViolation {
                    attempted_region: b,
                    home_region: d,
                },
            ) => a == b && c == d,
            (
                EffectData::AuditEntry { kind: a, count: c },
                EffectData::AuditEntry { kind: b, count: d },
            ) => a == b && c == d,
            // Unreachable: the discriminant guard above already excluded any
            // cross-variant pairing.
            _ => false,
        }
    }

    /// Cosmetic-tier projection. `spirit_pid` is the canonical cosmetic field
    /// (a logical Spirit identity, not a behavioral invariant). Variants with
    /// no cosmetic field are pure-invariant and vacuously satisfy the tier.
    ///
    /// This arm is NOT vacuous across the corpus: `HaltReceipt` and
    /// `CapabilityDenial` carry a real `spirit_pid`, and
    /// [`cosmetic_threshold_bites_through_invariant_green`] proves a corpus
    /// with enough `spirit_pid` drift drops cosmetic below 75 % → RED.
    fn cosmetic_matches(&self, other: &EffectData) -> bool {
        if self.discriminant() != other.discriminant() {
            return false;
        }
        match (self, other) {
            (
                EffectData::HaltReceipt { spirit_pid: a, .. },
                EffectData::HaltReceipt { spirit_pid: b, .. },
            ) => a == b,
            (
                EffectData::CapabilityDenial { spirit_pid: a, .. },
                EffectData::CapabilityDenial { spirit_pid: b, .. },
            ) => a == b,
            // FrameSequence / RegionViolation / AuditEntry carry no cosmetic
            // field — they are pure-invariant and so trivially pass the
            // cosmetic tier.
            _ => true,
        }
    }

    /// Canonical content rendering used as a TOTAL deterministic sort key so
    /// duplicate `(scenario, class)` keys pair correctly (no cross-wire).
    /// Two effects that are equal under the invariant+cosmetic projections
    /// render identically and therefore sort adjacent — the index-pairing in
    /// [`compare_effects`] becomes a bijection on equal multisets.
    fn content_key(&self) -> String {
        match self {
            EffectData::FrameSequence(v) => format!("frames:{}:{:016x}", v.len(), content_hash(v)),
            EffectData::HaltReceipt {
                halt_id,
                spirit_pid,
            } => {
                format!("halt:{halt_id}:{spirit_pid}")
            }
            EffectData::CapabilityDenial {
                error_kind,
                spirit_pid,
            } => {
                format!("cap:{error_kind}:{spirit_pid}")
            }
            EffectData::RegionViolation {
                attempted_region,
                home_region,
            } => format!("region:{attempted_region}:{home_region}"),
            EffectData::AuditEntry { kind, count } => format!("audit:{kind}:{count}"),
        }
    }
}

fn content_hash<T: std::fmt::Debug>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{value:?}").hash(&mut hasher);
    hasher.finish()
}

// ════════════════════════════════════════════════════════════════════════
// §2  Frame normalization + F3 exclusion allowlist
// ════════════════════════════════════════════════════════════════════════

/// The four domain fields the `maos:spirit@1.0` WIT projection does NOT
/// carry, so they cannot participate in a cross-form comparison. Enumerated
/// as an explicit `const` (not derived/looped) so:
///
/// 1. The excluded set is grep-able and reviewable.
/// 2. [`f3_allowlist_is_pinned_to_the_dropped_set`] pins the exact membership
///    — editing this `const` flips that test.
/// 3. [`normalize_ignores_exactly_the_f3_excluded_fields`] proves behaviorally
///    that `normalize` actually drops exactly these fields, and
///    [`all_invariant_fields_are_guarded_against_demotion`] proves the
///    preserved fields are NOT in this set (demoting one would flip RED).
///
/// Adding a field to this list requires editing this `const` AND keeping the
/// mutation tests green; the set and the behavior are coupled by design.
const F3_EXCLUDED_FIELDS: &[&str] = &["intent", "consent_envelope", "intent_lineage", "scope"];

/// The bridge-PRESERVED projection of an [`IacFrame`], with the per-run
/// nondeterministic fields (`frame_id`, `timestamp_ns`) zeroed so two
/// captures of the same logical effect compare equal regardless of when or
/// how they were emitted.
///
/// Only the fields that survive the `maos:spirit@1.0` lower/lift round-trip
/// are retained (see [`F3_EXCLUDED_FIELDS`]); the `scope` sub-field of a
/// `TaskAssign` payload is collapsed to the bridge's empty projection for a
/// fair native-vs-wasm comparison. The F3-excluded fields
/// (`intent`/`consent_envelope`/`intent_lineage`) are structurally absent
/// from this struct — there is no field through which they could survive.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedFrame {
    pub frame_id: [u8; 16],
    pub timestamp_ns: u64,
    pub logical_clock: u64,
    pub from: FrameAddress,
    pub to: SmallVec<[FrameAddress; 1]>,
    pub kind: FrameKind,
    pub payload: FramePayload,
    pub auto_marker: FrameOrigin,
}

/// Normalize a domain frame for cross-form comparison.
///
/// - `frame_id` → `[0u8; 16]` (a fresh random UUID per emission).
/// - `timestamp_ns` → `0` (wall-clock at emission).
/// - `scope` (inside a `TaskAssign` payload) → empty `Vec` (F3-excluded:
///   lossy across the WIT bridge).
/// - The F3-excluded top-level fields (`intent`, `consent_envelope`,
///   `intent_lineage`) are structurally dropped — `NormalizedFrame` has no
///   field for them.
/// - All other bridge-preserved fields (`logical_clock`, `from`, `to`,
///   `kind`, `payload` minus `scope`, `auto_marker`) are preserved exactly.
fn normalize(frame: &IacFrame) -> NormalizedFrame {
    // Collapse the F3-excluded `scope` sub-field to the bridge's projection.
    let payload = match &frame.payload {
        FramePayload::TaskAssign(ta) => FramePayload::TaskAssign(TaskAssignPayload {
            scope: Vec::new(),
            ..ta.clone()
        }),
        other => other.clone(),
    };

    NormalizedFrame {
        frame_id: [0u8; 16],
        timestamp_ns: 0,
        logical_clock: frame.logical_clock,
        from: frame.from.clone(),
        to: frame.to.clone(),
        kind: frame.kind,
        payload,
        auto_marker: frame.auto_marker,
    }
}

// ════════════════════════════════════════════════════════════════════════
// §3  Tiered comparator
// ════════════════════════════════════════════════════════════════════════

/// One row of the comparison table, for diagnosis.
#[derive(Debug, Clone)]
pub struct ComparisonDetail {
    pub scenario: String,
    pub class: Option<InvariantClass>,
    pub invariant_match: bool,
    pub cosmetic_match: bool,
    pub note: String,
}

/// The verdict returned by [`compare_effects`].
#[derive(Debug, Clone)]
pub struct TieredVerdict {
    /// Invariant-tier match percentage. The gate requires exactly `100.0`.
    pub invariant_match_pct: f64,
    /// Cosmetic-tier match percentage. The gate requires `>= 75.0`.
    pub cosmetic_match_pct: f64,
    /// `true` only when the invariant tier is `100.0`, the cosmetic tier is
    /// `>= 75.0`, AND the form pair is a genuine cross-form pair
    /// (`(Native, Wasm)` or `(Wasm, Native)`).
    pub passed: bool,
    /// The derived form identities of the two streams. A valid gate requires
    /// the two to differ (form-identity reflex).
    pub form_pair: (SpiritForm, SpiritForm),
    /// Per-position diagnosis.
    pub details: Vec<ComparisonDetail>,
}

/// Compare two captured effect streams and return a tiered verdict.
///
/// The two arguments are named `native` / `wasm` for readability, but the
/// form identity is DERIVED from each capture's `form` field (set by
/// whichever subprocess emitted it via [`capture_native`] / [`capture_wasm`]),
/// never trusted from the argument position.
///
/// # Fail-loud (no vacuous GREEN)
///
/// - An empty stream (absent / crashed-before-emit) → `passed = false`.
/// - A `FrameSequence(Vec::new())` (a form ran but emitted nothing) →
///   `passed = false`.
/// - A stream whose captures disagree on form internally (corruption) →
///   `passed = false` (a single live process emits one form).
/// - A same-form pair → `passed = false`.
pub fn compare_effects(native: &[CapturedEffect], wasm: &[CapturedEffect]) -> TieredVerdict {
    // ── Fail-loud gate 1: absent / crashed stream ──────────────────────
    // A form that produced zero effects was absent, crashed before emitting,
    // or never ran. None of those is a vacuous GREEN.
    if native.is_empty() || wasm.is_empty() {
        return TieredVerdict {
            invariant_match_pct: 0.0,
            cosmetic_match_pct: 0.0,
            passed: false,
            form_pair: (
                native.first().map(|e| e.form).unwrap_or(SpiritForm::Native),
                wasm.first().map(|e| e.form).unwrap_or(SpiritForm::Native),
            ),
            details: vec![ComparisonDetail {
                scenario: String::new(),
                class: None,
                invariant_match: false,
                cosmetic_match: false,
                note: format!(
                    "ABSENT FORM: native stream has {} effect(s), wasm stream has {} — \
                     a form that emitted nothing cannot be declared equivalent",
                    native.len(),
                    wasm.len()
                ),
            }],
        };
    }

    // ── Fail-loud gate 2: zero-frame FrameSequence ─────────────────────
    // Distinct from "absent": the form RAN but emitted no frames. Still not a
    // valid oracle.
    if has_zero_frames(native) || has_zero_frames(wasm) {
        return TieredVerdict {
            invariant_match_pct: 0.0,
            cosmetic_match_pct: 0.0,
            passed: false,
            form_pair: (unanimous_form(native), unanimous_form(wasm)),
            details: vec![ComparisonDetail {
                scenario: String::new(),
                class: None,
                invariant_match: false,
                cosmetic_match: false,
                note: "ZERO FRAMES: a form emitted an empty FrameSequence — \
                       the gate cannot confirm equivalence over no observations"
                    .to_string(),
            }],
        };
    }

    // ── Fail-loud gate 3: mixed-form stream ────────────────────────────
    // A single live process emits ONE form. A stream mixing Native and Wasm
    // captures is harness corruption (or a relabeling attempt) — never a
    // valid oracle. `unanimous_form` returns a placeholder for a mixed
    // stream; detect it explicitly so the note is honest.
    if !stream_form_is_unanimous(native) || !stream_form_is_unanimous(wasm) {
        let mixed_side = match (
            !stream_form_is_unanimous(native),
            !stream_form_is_unanimous(wasm),
        ) {
            (true, true) => "BOTH streams",
            (true, false) => "the NATIVE stream",
            (false, true) => "the WASM stream",
            _ => unreachable!("guarded above"),
        };
        return TieredVerdict {
            invariant_match_pct: 0.0,
            cosmetic_match_pct: 0.0,
            passed: false,
            form_pair: (unanimous_form(native), unanimous_form(wasm)),
            details: vec![ComparisonDetail {
                scenario: String::new(),
                class: None,
                invariant_match: false,
                cosmetic_match: false,
                note: format!(
                    "MIXED FORM STREAM: {mixed_side} mix Native and Wasm captures — \
                     a single live process emits exactly one form; a mixed stream \
                     is harness corruption, not a cross-form oracle"
                ),
            }],
        };
    }

    let native_form = unanimous_form(native);
    let wasm_form = unanimous_form(wasm);
    let form_pair = (native_form, wasm_form);

    // ── Form-identity reflex (Task 6) ──────────────────────────────────
    // Both streams MUST carry divergent live forms. Two Native streams (or
    // two Wasm) is a self-comparison: it trivially agrees and would give a
    // false GREEN. Reject outright.
    if native_form == wasm_form {
        return TieredVerdict {
            invariant_match_pct: 0.0,
            cosmetic_match_pct: 0.0,
            passed: false,
            form_pair,
            details: vec![ComparisonDetail {
                scenario: String::new(),
                class: None,
                invariant_match: false,
                cosmetic_match: false,
                note: format!(
                    "FORM-IDENTITY REFLEX: both streams report form {:?} — \
                     a cross-form oracle requires two DIFFERENT live forms",
                    native_form
                ),
            }],
        };
    }

    // ── Pair + compare ─────────────────────────────────────────────────
    // Sort each stream by (scenario, class, content) so pairing is robust to
    // both emission ordering AND duplicate (scenario, class) keys: two
    // effects that are equal under the invariant+cosmetic projections render
    // to the same content key and sort adjacent, so the index-pairing is a
    // bijection on equal multisets (no cross-wire false RED/GREEN).
    let mut n = native.to_vec();
    let mut w = wasm.to_vec();
    n.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
    w.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));

    let total = n.len().max(w.len());
    let mut invariant_ok = 0usize;
    let mut cosmetic_ok = 0usize;
    let mut details = Vec::with_capacity(total);

    for i in 0..total {
        match (n.get(i), w.get(i)) {
            (Some(a), Some(b)) => {
                let inv = a.data.invariant_matches(&b.data);
                let cos = a.data.cosmetic_matches(&b.data);
                if inv {
                    invariant_ok += 1;
                }
                if cos {
                    cosmetic_ok += 1;
                }
                let note = if a.scenario != b.scenario || a.invariant_class != b.invariant_class {
                    format!(
                        "key drift: native=({:?},{:?}) wasm=({:?},{:?})",
                        a.scenario, a.invariant_class, b.scenario, b.invariant_class
                    )
                } else if !inv {
                    format!(
                        "invariant divergence on {:?} for scenario {:?}",
                        a.invariant_class, a.scenario
                    )
                } else if !cos {
                    format!(
                        "cosmetic drift (non-blocking) on {:?} for scenario {:?}",
                        a.invariant_class, a.scenario
                    )
                } else {
                    format!(
                        "match on {:?} for scenario {:?}",
                        a.invariant_class, a.scenario
                    )
                };
                details.push(ComparisonDetail {
                    scenario: a.scenario.clone(),
                    class: Some(a.invariant_class),
                    invariant_match: inv,
                    cosmetic_match: cos,
                    note,
                });
            }
            (Some(only), None) | (None, Some(only)) => {
                // Length mismatch: an extra effect on one side. Counts as a
                // failure on BOTH tiers at this position.
                details.push(ComparisonDetail {
                    scenario: only.scenario.clone(),
                    class: Some(only.invariant_class),
                    invariant_match: false,
                    cosmetic_match: false,
                    note: format!(
                        "UNPAIRED effect: form produced an extra {:?} for \
                         scenario {:?} with no counterpart",
                        only.invariant_class, only.scenario
                    ),
                });
            }
            (None, None) => unreachable!("loop bounded by max(len)"),
        }
    }

    let invariant_match_pct = (invariant_ok as f64) * 100.0 / (total as f64);
    let cosmetic_match_pct = (cosmetic_ok as f64) * 100.0 / (total as f64);
    let passed = invariant_match_pct == 100.0 && cosmetic_match_pct >= 75.0;

    TieredVerdict {
        invariant_match_pct,
        cosmetic_match_pct,
        passed,
        form_pair,
        details,
    }
}

/// True iff every capture in a non-empty stream carries the SAME form. This
/// is the live-process derivation: a single subprocess emits one form, so a
/// stream with mixed forms is corruption, not a cross-form observation.
fn stream_form_is_unanimous(effects: &[CapturedEffect]) -> bool {
    match effects.first() {
        None => true,
        Some(first) => effects.iter().all(|e| e.form == first.form),
    }
}

/// Derive a stream's live form from its captures. A mixed or empty stream has
/// no single derivable form; we fall back to `Native` as a structural
/// placeholder and let the mixed-form / absent-form gates above produce the
/// honest verdict.
fn unanimous_form(effects: &[CapturedEffect]) -> SpiritForm {
    effects
        .first()
        .map(|e| e.form)
        .unwrap_or(SpiritForm::Native)
}

/// True if any effect is an empty `FrameSequence` — a form that ran but
/// emitted nothing.
fn has_zero_frames(effects: &[CapturedEffect]) -> bool {
    effects
        .iter()
        .any(|e| matches!(&e.data, EffectData::FrameSequence(v) if v.is_empty()))
}

/// Total deterministic sort key for robust index-independent pairing. The
/// content sub-key ([`EffectData::content_key`]) makes the sort a total order
/// over equal-multiset streams, so duplicate `(scenario, class)` keys cannot
/// cross-wire.
fn sort_key(e: &CapturedEffect) -> (&str, InvariantClass, String) {
    (e.scenario.as_str(), e.invariant_class, e.data.content_key())
}

// ════════════════════════════════════════════════════════════════════════
// §4  Subprocess runner helpers
// ════════════════════════════════════════════════════════════════════════

/// Path to a `target/<profile>/<name>` binary in the MAIN workspace target,
/// derived from this test's own location under `target/<profile>/deps/`.
fn target_binary(name: &str) -> PathBuf {
    let mut path = std::env::current_exe()
        .expect("current_exe")
        .parent()
        .expect("deps dir")
        .parent() // → target/<profile>
        .expect("profile dir")
        .to_path_buf();
    path.push(name);
    path
}

/// Fail LOUD if a required MAIN-workspace binary was not built (used for
/// `maos-wasm-runner`, a real `[[bin]]` of this package built by the test
/// run). A missing binary is a test-environment defect, never a silent skip.
fn require_binary(name: &str) -> PathBuf {
    let path = target_binary(name);
    assert!(
        path.exists(),
        "required binary `{name}` not found at {} — it has not been built yet \
         (the runner is `maos-wasm-runner`, a [[bin]] of this package). Run \
         `cargo test -p maos-wasm-host` (not `--lib`) so cargo builds every \
         [[bin]] target alongside the test harness.",
        path.display()
    );
    path
}

/// Resolve the `equiv-native-twin` binary wherever it actually lives and
/// build it on demand if absent.
///
/// The native twin is a STANDALONE `[workspace]` (crypto-free, dev-only per
/// D11), so `cargo test -p maos-wasm-host` does NOT build it. It is produced
/// by `cargo build [--release] --manifest-path .../native-twin/Cargo.toml`,
/// which emits `native-twin/target/<profile>/equiv-native-twin`. This helper
/// discovers that location (release, then debug), falls back to a legacy
/// main-workspace `target/<profile>/` copy, and finally builds on demand — so
/// the twin is always reachable from the test regardless of which build path
/// staged it.
fn ensure_native_twin_binary() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let twin_dir = format!("{manifest_dir}/guests/equiv-fixture/native-twin");

    // 1. Standalone-workspace builds (where the manifest-path build emits it).
    let release = PathBuf::from(format!("{twin_dir}/target/release/equiv-native-twin"));
    if release.exists() {
        return release;
    }
    let debug = PathBuf::from(format!("{twin_dir}/target/debug/equiv-native-twin"));
    if debug.exists() {
        return debug;
    }

    // 2. Legacy: main-workspace target (present in some local setups).
    let main_target = target_binary("equiv-native-twin");
    if main_target.exists() {
        return main_target;
    }

    // 3. Build on demand into the standalone workspace (release profile to
    //    match the CI contract).
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = format!("{twin_dir}/Cargo.toml");
    let status = Command::new(&cargo)
        .args(["build", "--release", "--manifest-path", &manifest])
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke {cargo} to build equiv-native-twin: {e}"));
    assert!(
        status.success(),
        "on-demand build of equiv-native-twin failed (status {status}); \
         run `cargo build --release --manifest-path {manifest}` manually"
    );
    assert!(
        release.exists(),
        "on-demand build reported success but did not produce {release:?}"
    );
    release
}

/// Resolve the committed WASM component fixture for an equivalence scenario
/// `variant`. Committed artifacts under `tests/fixtures/wasm/`, never
/// `target/` (gitignored, profile/triple-dependent, non-canonical).
///
/// Each variant resolves to its DEDICATED component only — there is NO silent
/// fallback to `echo_spirit_component.wasm`. The identity/divergent/cosmetic
/// fixtures are distinct `maos:spirit@1.0` components; the identity test
/// calls [`require_component`] exactly like the divergent/cosmetic tests, so a
/// missing or semantically-drifting identity fixture fails LOUD (never passes
/// for the wrong reason over an unrelated echo fixture).
fn component_fixture_path(variant: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dir = format!("{manifest_dir}/../../tests/fixtures/wasm");
    match variant {
        "identity" => format!("{dir}/equiv_identity_spirit_component.wasm"),
        "divergent" => format!("{dir}/equiv_divergent_spirit_component.wasm"),
        "cosmetic" => format!("{dir}/equiv_cosmetic_spirit_component.wasm"),
        other => panic!("unknown component fixture variant: {other}"),
    }
}

/// Fail LOUD if a required WASM component fixture is missing. A missing
/// fixture is a test-environment defect, never a silent skip — reporting a
/// false GREEN on a gate that never ran the WASM form would defeat the oracle.
fn require_component(path: &str) {
    assert!(
        std::path::Path::new(path).exists(),
        "required WASM component fixture not found at {path} — it has not been \
         built/committed yet (Story 11.1b Tasks 1-3 stage the \
         equiv-{{identity,divergent,cosmetic}}-spirit components at \
         tests/fixtures/wasm/<name>_component.wasm)."
    );
}

/// Drive a list of frames through a subprocess that speaks ADR-032
/// (Content-Length + CBOR) over stdio and collect every emitted frame back.
///
/// `binary` / `args` select the subprocess; frames are CBOR-encoded, framed,
/// and written on a background thread (avoiding pipe-buffer deadlock when the
/// guest emits before consuming all input). STDERR is drained concurrently on
/// its own thread — without that, a chatty guest/runner can fill the stderr
/// pipe buffer and deadlock the child while the parent blocks reading stdout.
/// Captured stderr is surfaced in the failure message for diagnosis.
fn drive_subprocess(binary: &Path, args: &[&str], input_frames: &[IacFrame]) -> Vec<IacFrame> {
    let mut child = Command::new(binary)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {} {:?}: {e}", binary.display(), args));

    let stdin = child.stdin.take().expect("stdin pipe");
    let stdout = child.stdout.take().expect("stdout pipe");
    let stderr = child.stderr.take().expect("stderr pipe");

    // Writer thread: encode + frame + write all input, then drop stdin to
    // signal EOF (the runner treats stdin EOF as a voluntary Halt).
    let to_send = input_frames.to_vec();
    let writer = std::thread::spawn(move || {
        let mut writer = BufWriter::new(stdin);
        for frame in &to_send {
            let bytes = codec::encode_cbor(frame).expect("encode input frame");
            codec::write_frame(&mut writer, &bytes).expect("write input frame");
        }
        // Drop `writer` (and the stdin it owns) → EOF.
        drop(writer);
    });

    // Stderr drain thread — keeps the stderr pipe empty so the child never
    // blocks on a full stderr buffer while the parent reads stdout.
    let stderr_handle = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut reader = std::io::BufReader::new(stderr);
        let _ = std::io::Read::read_to_string(&mut reader, &mut buf);
        buf
    });

    // Reader: drain stdout frame-by-frame until clean EOF.
    let mut reader = BufReader::new(stdout);
    let mut emitted = Vec::new();
    while let Some(bytes) = codec::read_frame(&mut reader).expect("read output frame") {
        let frame: IacFrame = codec::decode_cbor(&bytes).expect("decode output frame");
        emitted.push(frame);
    }

    writer.join().expect("writer thread must not panic");
    let stderr_output = stderr_handle
        .join()
        .expect("stderr drain thread must not panic");

    let status = child.wait().expect("wait for subprocess");
    assert!(
        status.success(),
        "subprocess {} {:?} exited {status:?}\n--- stderr ---\n{stderr_output}",
        binary.display(),
        args
    );

    emitted
}

/// Run the NATIVE twin form: `equiv-native-twin --mode <mode>`.
///
/// Feeds `input_frames` over stdin, returns every emitted frame. Fails loud
/// if the `equiv-native-twin` binary is absent — resolved (and built on
/// demand) by [`ensure_native_twin_binary`].
fn run_native_twin(mode: &str, input_frames: &[IacFrame]) -> Vec<IacFrame> {
    let binary = ensure_native_twin_binary();
    drive_subprocess(&binary, &["--mode", mode], input_frames)
}

/// WASM fuel budget for the equivalence runner — the spec's nominal value
/// (`maos-wasm-runner --component <path> --fuel 1000000`).
///
/// This is the spec-faithful, tighter AC4 defense-in-depth bound (T2 sandbox
/// is the strict backstop). The cosmetic-control fixture realizes its latency
/// as a fuel-billing spin (wasm32-unknown-unknown has no WASI clock), so that
/// fixture's spin is calibrated (`COSMETIC_SPIN_ITERS = 10_000`, documented in
/// `cosmetic-guest/src/lib.rs`) to fit within this budget with headroom for the
/// frame round-trip — verified end-to-end through the real runner.
const WASM_FUEL_BUDGET: &str = "1000000";

/// Run the WASM form: `maos-wasm-runner --component <path> --fuel <budget>`.
///
/// Feeds `input_frames` over stdin, returns every emitted frame. Fails loud
/// if the `maos-wasm-runner` binary is absent or the guest traps (e.g.
/// out-of-fuel).
fn run_wasm_form(component_path: &str, input_frames: &[IacFrame]) -> Vec<IacFrame> {
    let binary = require_binary("maos-wasm-runner");
    drive_subprocess(
        &binary,
        &["--component", component_path, "--fuel", WASM_FUEL_BUDGET],
        input_frames,
    )
}

/// Run the NATIVE twin and return the result already stamped as a Native
/// `FrameSequence` capture. The form identity is derived HERE — from the live
/// `equiv-native-twin` subprocess this runner spawns — so the calling test can
/// never mislabel a real native emission (there is no `SpiritForm` argument in
/// the real-subprocess path). This is the form-identity reflex's source of
/// truth (D8): the live emitting process, not a caller label.
fn run_native_twin_capturing(
    scenario: &str,
    mode: &str,
    input_frames: &[IacFrame],
) -> CapturedEffect {
    let frames = run_native_twin(mode, input_frames);
    capture_native_frame_sequence(scenario, frames)
}

/// Run the WASM form and return the result already stamped as a Wasm
/// `FrameSequence` capture — the form derived from the live
/// `maos-wasm-runner` subprocess. See [`run_native_twin_capturing`].
fn run_wasm_form_capturing(
    scenario: &str,
    component_path: &str,
    input_frames: &[IacFrame],
) -> CapturedEffect {
    let frames = run_wasm_form(component_path, input_frames);
    capture_wasm_frame_sequence(scenario, frames)
}

// ════════════════════════════════════════════════════════════════════════
// §5  Capture/fixture helpers
// ════════════════════════════════════════════════════════════════════════

/// Stamp a capture as emitted by the NATIVE live process (the
/// `equiv-native-twin` subprocess). The form is bound to this helper — there
/// is no free `SpiritForm` argument, so a caller cannot relabel a capture.
fn capture_native(scenario: &str, class: InvariantClass, data: EffectData) -> CapturedEffect {
    CapturedEffect {
        form: SpiritForm::Native,
        scenario: scenario.to_string(),
        invariant_class: class,
        data,
    }
}

/// Stamp a capture as emitted by the WASM live process (the
/// `maos-wasm-runner` subprocess). See [`capture_native`].
fn capture_wasm(scenario: &str, class: InvariantClass, data: EffectData) -> CapturedEffect {
    CapturedEffect {
        form: SpiritForm::Wasm,
        scenario: scenario.to_string(),
        invariant_class: class,
        data,
    }
}

/// Build a `FrameSequence` effect from emitted frames.
fn effect_frame_sequence(frames: Vec<IacFrame>) -> EffectData {
    EffectData::FrameSequence(frames.iter().map(normalize).collect())
}

/// Wrap a frame sequence emitted by the NATIVE form for `scenario`.
fn capture_native_frame_sequence(scenario: &str, frames: Vec<IacFrame>) -> CapturedEffect {
    capture_native(
        scenario,
        InvariantClass::FrameSequence,
        effect_frame_sequence(frames),
    )
}

/// Wrap a frame sequence emitted by the WASM form for `scenario`.
fn capture_wasm_frame_sequence(scenario: &str, frames: Vec<IacFrame>) -> CapturedEffect {
    capture_wasm(
        scenario,
        InvariantClass::FrameSequence,
        effect_frame_sequence(frames),
    )
}

/// Build a `HaltReceipt` effect from a REAL [`HaltReceipt`]. The invariant
/// projection is the halt identity (`halt_id`); the cosmetic projection is
/// the logical `spirit_pid`.
fn effect_halt(receipt: &HaltReceipt) -> EffectData {
    EffectData::HaltReceipt {
        halt_id: receipt.halt_id.as_str().to_string(),
        spirit_pid: receipt.spirit_pid,
    }
}

/// Build a `CapabilityDenial` effect from a REAL [`CapError`]. The invariant
/// projection is the denial taxonomy (`error_kind`, derived from the real
/// `CapError` variant); `spirit_pid` is the cosmetic projection.
fn effect_capability_denial(err: &CapError, spirit_pid: u32) -> EffectData {
    EffectData::CapabilityDenial {
        error_kind: cap_error_kind(err).to_string(),
        spirit_pid,
    }
}

/// Stable invariant tag for a real [`CapError`] variant — the kernel's
/// capability-denial taxonomy (the thing that must match cross-form).
fn cap_error_kind(err: &CapError) -> &'static str {
    match err {
        CapError::CryptoFailed(_) => "CryptoFailed",
        CapError::UnknownToken => "UnknownToken",
        CapError::Expired => "Expired",
        CapError::Revoked => "Revoked",
        CapError::SpiritIdMismatch => "SpiritIdMismatch",
        CapError::SignatureMismatch => "SignatureMismatch",
        CapError::PostureMismatch => "PostureMismatch",
        CapError::ContextExhausted { .. } => "ContextExhausted",
        CapError::PolicyDenied => "PolicyDenied",
    }
}

/// Build a `RegionViolation` effect from REAL [`Region`]s. Both the attempted
/// and home regions are invariant (region-pin is kernel/operator config,
/// invariant to form by construction).
fn effect_region_violation(attempted: &Region, home: &Region) -> EffectData {
    EffectData::RegionViolation {
        attempted_region: attempted.as_str().to_string(),
        home_region: home.as_str().to_string(),
    }
}

/// Build an `AuditEntry` effect. `kind` is a real kernel audit kind string;
/// `count` is the observed row count. Both are invariant.
fn effect_audit(kind: &str, count: usize) -> EffectData {
    EffectData::AuditEntry {
        kind: kind.to_string(),
        count,
    }
}

/// A deterministic identity-probe frame for the equivalence corpus.
fn sample_identity_frame() -> IacFrame {
    IacFrame {
        frame_id: [0x11; 16],
        timestamp_ns: 9_876_543,
        logical_clock: 7,
        from: FrameAddress {
            spirit_id: "equiv-native".into(),
            host_id: None,
            role: None,
        },
        to: smallvec::smallvec![FrameAddress {
            spirit_id: "equiv-peer".into(),
            host_id: None,
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "equiv-gate identity probe".to_string(),
            scope: Vec::new(),
            success_criteria: "both forms emit identical normalized frames".to_string(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

// ════════════════════════════════════════════════════════════════════════
// §6  Anti-canned tripwire (feature-gated)
// ════════════════════════════════════════════════════════════════════════

/// Perturb a captured invariant-tier effect AFTER capture but BEFORE
/// comparison. Only compiled under `--features equiv-fault-inject`.
///
/// This exists to prove the comparator is not canned: if you perturb the
/// data, the verdict must move. A harness that ignored its inputs and always
/// returned GREEN would pass every canned assertion but fail here.
///
/// The injection covers EVERY invariant class (not just frame-sequence) and,
/// for the frame's `logical_clock`, uses [`u64::saturating_add`] — the SAME
/// arithmetic the real divergent transform (`equiv_fixture_logic`) uses — so
/// the tripwire cannot drift out of sync with the behavior under test. The
/// `u64::MAX` ceiling (where saturating diverge is a no-op) is exercised
/// separately by [`fault_injection_boundary_moves_under_distinct_perturbation`]
/// via a guaranteed-distinct perturbation.
#[cfg(feature = "equiv-fault-inject")]
fn inject_invariant_fault(effect: &mut CapturedEffect) {
    match &mut effect.data {
        EffectData::FrameSequence(frames) => {
            if let Some(first) = frames.first_mut() {
                first.logical_clock = first.logical_clock.saturating_add(1);
            }
        }
        EffectData::HaltReceipt { halt_id, .. } => {
            // Flip the halt identity — a real divergence on an invariant field.
            *halt_id = format!("{halt_id}<<fault>>");
        }
        EffectData::CapabilityDenial { error_kind, .. } => {
            *error_kind = format!("{error_kind}<<fault>>");
        }
        EffectData::RegionViolation {
            attempted_region, ..
        } => {
            *attempted_region = format!("{attempted_region}<<fault>>");
        }
        EffectData::AuditEntry { count, .. } => {
            *count = count.wrapping_add(1);
        }
    }
}

/// A perturbation that is GUARANTEED to change the invariant projection even
/// at the `u64::MAX` ceiling (where [`inject_invariant_fault`]'s saturating
/// `logical_clock += 1` is a no-op). Proves the comparator is not canned at
/// the boundary — the only place saturating diverge cannot move the number.
#[cfg(feature = "equiv-fault-inject")]
fn inject_invariant_fault_distinct(effect: &mut CapturedEffect) {
    // For frame-sequences the saturating primary injection is a no-op at
    // u64::MAX; force a guaranteed-distinct value there. For every other
    // class the primary injection already moves the invariant projection, so
    // delegate to it.
    if let EffectData::FrameSequence(frames) = &mut effect.data {
        if let Some(first) = frames.first_mut() {
            // MAX → 0; 0 → 1; otherwise halve — always distinct from the
            // current value, so the comparator cannot be canned anywhere
            // (including the u64::MAX ceiling where saturating diverge is a
            // no-op).
            first.logical_clock = if first.logical_clock == u64::MAX {
                0
            } else if first.logical_clock == 0 {
                1
            } else {
                first.logical_clock / 2
            };
        }
    } else {
        inject_invariant_fault(effect);
    }
}

// ════════════════════════════════════════════════════════════════════════
// §7  Tests
// ════════════════════════════════════════════════════════════════════════

// NOTE: the three pipeline tests below (identity / divergent / cosmetic) drive
// REAL subprocesses. The native twin is always the identity-mode REFERENCE
// (echo); the WASM form carries the variant under test. They need the
// `equiv-native-twin` (resolved + built on demand by `ensure_native_twin_binary`)
// + `maos-wasm-runner` binaries and the committed equiv WASM components at
// `tests/fixtures/wasm/equiv_*_component.wasm` (Story 11.1b Tasks 1-3). The
// comparator-logic + per-class coverage tests exercise the oracle directly
// with synthetic captures built from REAL domain types.

/// Happy path: the identity Spirit, run through both forms, produces a 100 %
/// invariant match and a cross-form GREEN. Both forms run identity/echo
/// logic — the native twin in `identity` mode, the WASM form via the identity
/// component — so their normalized frames must agree exactly.
#[test]
fn identity_spirit_both_forms_100pct_invariant() {
    let component = component_fixture_path("identity");
    require_component(&component);
    let inputs = vec![sample_identity_frame()];

    // The form identity on each capture is derived from the LIVE subprocess
    // (the runner stamps it) — never from a caller-supplied label.
    let native = vec![run_native_twin_capturing("identity", "identity", &inputs)];
    let wasm = vec![run_wasm_form_capturing("identity", &component, &inputs)];

    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed,
        "identity spirit must be GREEN across forms: {verdict:?}"
    );
    assert_eq!(verdict.invariant_match_pct, 100.0);
    // Form identity must be a genuine cross-form pair.
    assert_ne!(verdict.form_pair.0, verdict.form_pair.1);
}

/// Proven-RED: the divergent WASM component mutates an invariant field
/// (`logical_clock += 1`) while the native twin (identity mode) echoes
/// unchanged. The WASM form is the mutant — the gate MUST catch the
/// invariant divergence and flip RED.
#[test]
fn divergent_spirit_causes_invariant_red() {
    let component = component_fixture_path("divergent");
    require_component(&component);
    let inputs = vec![sample_identity_frame()];

    // Native twin = identity reference (echo); WASM form = divergent mutant.
    // Form identity is stamped by each runner (the live subprocess), not a label.
    let native = vec![run_native_twin_capturing("divergent", "identity", &inputs)];
    let wasm = vec![run_wasm_form_capturing("divergent", &component, &inputs)];

    let verdict = compare_effects(&native, &wasm);
    assert!(
        !verdict.passed,
        "a divergent invariant field MUST be RED: {verdict:?}"
    );
    assert!(
        verdict.invariant_match_pct < 100.0,
        "invariant tier must drop below 100% on divergence"
    );
}

/// Cosmetic control: the cosmetic WASM component adds latency but echoes the
/// frame unchanged, while the native twin (identity mode) echoes immediately.
/// Latency is invisible to the frame-comparison oracle, so the gate stays
/// GREEN — the cosmetic tier tolerates non-behavioral drift.
#[test]
fn cosmetic_only_stays_green() {
    let component = component_fixture_path("cosmetic");
    require_component(&component);
    let inputs = vec![sample_identity_frame()];

    // Form identity is stamped by each runner (the live subprocess), not a label.
    let native = vec![run_native_twin_capturing("cosmetic", "identity", &inputs)];
    let wasm = vec![run_wasm_form_capturing("cosmetic", &component, &inputs)];

    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed,
        "a cosmetic-only (latency) divergence must stay GREEN: {verdict:?}"
    );
    assert_eq!(verdict.invariant_match_pct, 100.0);
}

/// Form-identity reflex: two streams that share the same form are a
/// self-comparison and MUST be rejected — never a vacuous GREEN.
#[test]
fn same_form_pair_rejected() {
    let frame = sample_identity_frame();
    // Both streams stamped Native (via the native capture helper) → the
    // form-identity reflex fires.
    let native = vec![capture_native_frame_sequence("self", vec![frame.clone()])];
    let also_native = vec![capture_native_frame_sequence("self", vec![frame])];

    let verdict = compare_effects(&native, &also_native);
    assert!(
        !verdict.passed,
        "same-form pair must be REJECTED by the form-identity reflex: {verdict:?}"
    );
    assert_eq!(verdict.form_pair.0, SpiritForm::Native);
    assert_eq!(verdict.form_pair.1, SpiritForm::Native);
}

/// A stream that mixes Native and Wasm captures is corruption (a single live
/// process emits one form) and MUST be rejected — never a vacuous GREEN. This
/// hardens the form-identity derivation beyond "trust the first label".
#[test]
fn mixed_form_stream_rejected() {
    let frame = sample_identity_frame();
    let mixed = vec![
        capture_native_frame_sequence("mix", vec![frame.clone()]),
        capture_wasm_frame_sequence("mix", vec![frame]),
    ];
    let wasm = vec![capture_wasm_frame_sequence(
        "mix",
        vec![sample_identity_frame()],
    )];

    let verdict = compare_effects(&mixed, &wasm);
    assert!(
        !verdict.passed,
        "a mixed-form stream MUST fail loud, not pass vacuously: {verdict:?}"
    );
    assert!(
        verdict
            .details
            .iter()
            .any(|d| d.note.contains("MIXED FORM STREAM")),
        "expected a MIXED FORM STREAM diagnosis: {verdict:?}"
    );
}

/// No vacuous skip on an absent form: a stream that emitted nothing (absent /
/// crashed) fails loud rather than passing on an empty comparison.
#[test]
fn absent_form_fails_loud() {
    let frame = sample_identity_frame();
    let native = vec![capture_native_frame_sequence("absent", vec![frame])];
    let wasm: Vec<CapturedEffect> = Vec::new(); // absent / crashed

    let verdict = compare_effects(&native, &wasm);
    assert!(
        !verdict.passed,
        "an absent form MUST fail loud, not pass vacuously: {verdict:?}"
    );
}

/// No vacuous skip on zero frames: a form that ran but emitted an empty
/// `FrameSequence` fails loud.
#[test]
fn zero_frames_fails_loud() {
    let frame = sample_identity_frame();
    let native = vec![capture_native_frame_sequence("zero", Vec::new())]; // ran, emitted nothing
    let wasm = vec![capture_wasm_frame_sequence("zero", vec![frame])];

    let verdict = compare_effects(&native, &wasm);
    assert!(
        !verdict.passed,
        "a zero-frame form MUST fail loud, not pass vacuously: {verdict:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// §7a  In-process KERNEL oracle — the 4/5 invariant classes observed for real (finding 1)
// ────────────────────────────────────────────────────────────────────────
//
// Finding #1 required the halt/capability-denial/region-pin/audit classes to
// be OBSERVED through the real kernel (KernelCtx / maos_audit), not merely
// constructed as comparator-domain types. The `kernel_oracle` module below
// drives the ACTUAL in-process kernel adapters (`maos-kernel-core` is already a
// dev-dependency — no production API change, no mocks):
//   • HALT       → `invoke_halt` returns a real `HaltReceipt` (the kernel's
//                   single owner of the TL kind=3 row + journal entry).
//   • CAPABILITY → `CapabilityRegistryAdapter::verify_and_audit` returns a real
//                   `Result<(), CapError>` decision.
//   • REGION-PIN → `write_entry_point::enforce_region` returns a real
//                   `Err(RegionError::ERegionViolation)` at the write chokepoint.
//   • AUDIT      → the real `cap_audit` channel carries the
//                   `capability.invocation` event, drained in-process
//                   (`try_recv` is synchronous — no tokio runtime needed).
//
// The kernel decision is form-agnostic by construction (D12/F4: it DERIVES
// from spirit_pid / scope / home_region, never from the frame's in-band
// fields), so observing it once and attributing it to each form yields a
// genuine cross-form invariant match — and a different real kernel input
// (revoked token / foreign region / distinct halt) flips the gate RED.

fn sample_halt_receipt() -> HaltReceipt {
    HaltReceipt::new(
        HaltId::new("halt-equiv-probe").expect("valid halt id"),
        1_700_000_000_000,
        4242,
        0,
        [0x22; 16],
    )
}

/// The real audit `kind` strings the kernel TL writer emits (F4 observation
/// surface). Pinned by [`kernel_audit_kinds_are_real`] so the oracle's kind
/// vocabulary cannot silently drift from the kernel's.
const AUDIT_KIND_EPISTEMIC_HALT: &str = "epistemic.halt";
const AUDIT_KIND_CAPABILITY_INVOCATION: &str = "capability.invocation";
const AUDIT_KIND_SANDBOX_BLOCK: &str = "sandbox.block";

/// Capability-decision projection from a real `verify_and_audit` result:
/// `Ok` → "Allowed"; `Err(e)` → the real `CapError` variant tag.
fn effect_capability_decision(res: &Result<(), CapError>, spirit_pid: u32) -> EffectData {
    let error_kind = match res {
        Ok(()) => "Allowed".to_string(),
        Err(e) => cap_error_kind(e).to_string(),
    };
    EffectData::CapabilityDenial {
        error_kind,
        spirit_pid,
    }
}

mod kernel_oracle {
    //! Real in-process kernel observation of the four non-FrameSequence
    //! invariant classes. See the §7a header — every observer here calls the
    //! kernel's own public adapter APIs.
    use super::*;
    use std::sync::Arc;

    use maos_domain::frame::EpistemicHaltPayload;
    use maos_domain::invariants::i1::{IntentClass, Scope};
    use maos_domain::invariants::i9::SandboxTier;
    use maos_domain::ports::capability::CapError;
    use maos_domain::ports::CapabilityRegistryPort;
    use maos_domain::region::{Region, RegionError};
    use maos_kernel_core::api::RingCryptoProvider;
    use maos_kernel_core::capability::{
        cap_audit, cap_policy, cap_quota, cap_tokens, CapabilityRegistryAdapter, WorkingMemoryStore,
    };
    use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
    use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
    use maos_kernel_core::journal::JournalAdapter;
    use maos_kernel_core::memory::write_entry_point::{enforce_region, WriteEntryPoint};
    use maos_kernel_core::telemetry::TelemetryStreamAdapter;

    const POLICY_SPIRIT_PID: u32 = 7;
    const POSTURE: [u8; 32] = [0xAB; 32];
    const SANDBOX: SandboxTier = SandboxTier(0);
    const HOME_REGION_TAG: &str = "eu";
    const FOREIGN_REGION_TAG: &str = "us";

    /// Build a REAL capability registry (RingCryptoProvider + a PolicyTable
    /// admitting spirit 7 / `FsRead{/tmp}`) and return the audit receiver so
    /// the caller can drain real `CapAuditEvent`s. Mirrors the kernel-core
    /// `cap_registry_integration::make_adapter` wiring with public types only.
    pub fn cap_registry() -> (
        CapabilityRegistryAdapter,
        tokio::sync::mpsc::Receiver<cap_audit::CapAuditEvent>,
    ) {
        cap_tokens::init_monotonic_base();
        let policy = cap_policy::PolicyTable::new();
        {
            let mut inner = cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(
                POLICY_SPIRIT_PID,
                cap_policy::ManifestCapabilityScope {
                    scopes: vec![Scope::FsRead {
                        subtree: "/tmp".into(),
                    }],
                    declared_tier: SandboxTier(0),
                    trust_tier: cap_policy::decision::TrustTier::Verified,
                },
            );
            policy.update(inner);
        }
        let (audit_tx, audit_rx) = cap_audit::channel();
        let adapter = CapabilityRegistryAdapter::new(
            Arc::new(RingCryptoProvider),
            cap_tokens::Ed25519SigningKey::new([0u8; 32]),
            0,
            Arc::new(policy),
            audit_tx,
            cap_quota::CapQuotaTracker::new(),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        );
        (adapter, audit_rx)
    }

    /// Issue a real token for the policy spirit, optionally revoke it, then
    /// `verify_and_audit`. Returns the REAL kernel decision.
    pub fn capability_decision(
        adapter: &CapabilityRegistryAdapter,
        revoke: bool,
    ) -> Result<(), CapError> {
        let token = adapter
            .issue(
                POLICY_SPIRIT_PID,
                Scope::FsRead {
                    subtree: "/tmp".into(),
                },
                60,
                POSTURE,
                IntentClass::Standard,
            )
            .expect("issue token for policy spirit");
        if revoke {
            adapter.revoke(token.token_id).expect("revoke token");
        }
        adapter.verify_and_audit(&token, POSTURE, SANDBOX)
    }

    /// Perform `n` real `verify_and_audit` calls and drain the audit channel,
    /// returning the count of real `capability.invocation` audit events the
    /// kernel emitted for this batch. `try_recv` is synchronous (no runtime).
    pub fn capability_invocation_audit_count(
        adapter: &CapabilityRegistryAdapter,
        rx: &mut tokio::sync::mpsc::Receiver<cap_audit::CapAuditEvent>,
        n: usize,
    ) -> usize {
        for _ in 0..n {
            let token = adapter
                .issue(
                    POLICY_SPIRIT_PID,
                    Scope::FsRead {
                        subtree: "/tmp".into(),
                    },
                    60,
                    POSTURE,
                    IntentClass::Standard,
                )
                .expect("issue token");
            let _ = adapter.verify_and_audit(&token, POSTURE, SANDBOX);
        }
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        count
    }

    /// Invoke a REAL halt via the kernel's single-owner path (TL kind=3 row +
    /// journal entry + `HaltReceipt`) and return the receipt.
    pub fn halt_receipt(halt_suffix: &str) -> HaltReceipt {
        let tmp = tempfile::TempDir::new().expect("journal tmpdir");
        let journal =
            JournalAdapter::open(&tmp.path().join("journal.ndjson")).expect("open journal");
        let tl = TransparencyLogAdapter::open_in_memory(0);
        let registry = HaltRegistry::new();
        let payload = EpistemicHaltPayload::new(
            format!("halt-equiv-{halt_suffix}"),
            "claim.security_vulnerability".to_string(),
            0.9,
            Some(0.5),
            "epistemic_policy.v1".to_string(),
            "equiv-oracle".to_string(),
        )
        .expect("valid halt payload");
        invoke_halt(
            &tl,
            &journal,
            &registry,
            payload,
            POLICY_SPIRIT_PID,
            "equiv-spirit",
            0,
        )
        .expect("invoke_halt produces a receipt")
    }

    /// Observe a REAL region-pin decision at the write chokepoint and project
    /// it to the comparator's `RegionViolation` effect. `foreign=false` → a
    /// home write (Ok, attempted==home); `foreign=true` → a cross-region write
    /// → `ERegionViolation{expected=home, found=foreign}`.
    pub fn region_violation_effect(foreign: bool) -> EffectData {
        let home = Region::canonicalize(HOME_REGION_TAG).expect("home region");
        let entry = if foreign {
            WriteEntryPoint::ReplayApply {
                source_region: Some(
                    Region::canonicalize(FOREIGN_REGION_TAG).expect("foreign region"),
                ),
            }
        } else {
            WriteEntryPoint::DirectWrite
        };
        match enforce_region(&entry, Some(&home)) {
            Ok(()) => EffectData::RegionViolation {
                attempted_region: home.as_str().to_string(),
                home_region: home.as_str().to_string(),
            },
            Err(RegionError::ERegionViolation {
                expected, found, ..
            }) => EffectData::RegionViolation {
                attempted_region: found,
                home_region: expected,
            },
            Err(other) => panic!("unexpected region decision: {other:?}"),
        }
    }
}

/// The audit `kind` vocabulary matches the kernel TL writer's real tags.
#[test]
fn kernel_audit_kinds_are_real() {
    assert_eq!(AUDIT_KIND_EPISTEMIC_HALT, "epistemic.halt");
    assert_eq!(AUDIT_KIND_CAPABILITY_INVOCATION, "capability.invocation");
    assert_eq!(AUDIT_KIND_SANDBOX_BLOCK, "sandbox.block");
}

/// HALT class observed through the real in-process kernel: `invoke_halt`
/// produces a real `HaltReceipt`. The kernel decision is form-agnostic, so the
/// same halt identity attributed to each form is GREEN; a different real halt
/// → RED.
#[test]
fn halt_observed_via_in_process_kernel() {
    let receipt = kernel_oracle::halt_receipt("probe");
    let native = vec![capture_native(
        "halt-ker",
        InvariantClass::Halt,
        effect_halt(&receipt),
    )];
    let wasm = vec![capture_wasm(
        "halt-ker",
        InvariantClass::Halt,
        effect_halt(&receipt),
    )];
    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed && verdict.invariant_match_pct == 100.0,
        "identical real halt receipt must be GREEN across forms: {verdict:?}"
    );

    // A DIFFERENT real halt (distinct halt_id from a fresh invoke_halt) → RED.
    let other = kernel_oracle::halt_receipt("divergent");
    assert_ne!(other.halt_id, receipt.halt_id, "sanity: distinct halts");
    let wasm_div = vec![capture_wasm(
        "halt-ker",
        InvariantClass::Halt,
        effect_halt(&other),
    )];
    let red = compare_effects(&native, &wasm_div);
    assert!(
        !red.passed && red.invariant_match_pct < 100.0,
        "a divergent real halt identity MUST be RED: {red:?}"
    );
}

/// CAPABILITY-DENIAL class observed through the real in-process kernel:
/// `verify_and_audit` returns a real `Result<(), CapError>`. A valid token →
/// "Allowed" (GREEN across forms); a revoked token → "Revoked" (RED).
#[test]
fn capability_decision_observed_via_in_process_kernel() {
    let (adapter, _rx) = kernel_oracle::cap_registry();
    let allowed = kernel_oracle::capability_decision(&adapter, false);
    assert!(allowed.is_ok(), "sanity: valid token verifies: {allowed:?}");

    let native = vec![capture_native(
        "cap-ker",
        InvariantClass::CapabilityDenial,
        effect_capability_decision(&allowed, 7),
    )];
    let wasm = vec![capture_wasm(
        "cap-ker",
        InvariantClass::CapabilityDenial,
        effect_capability_decision(&allowed, 7),
    )];
    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed && verdict.invariant_match_pct == 100.0,
        "identical real capability decision must be GREEN: {verdict:?}"
    );

    // A REVOKED token → a different real kernel decision → RED.
    let revoked = kernel_oracle::capability_decision(&adapter, true);
    assert!(
        revoked.is_err(),
        "sanity: revoked token denies: {revoked:?}"
    );
    let wasm_div = vec![capture_wasm(
        "cap-ker",
        InvariantClass::CapabilityDenial,
        effect_capability_decision(&revoked, 7),
    )];
    let red = compare_effects(&native, &wasm_div);
    assert!(
        !red.passed && red.invariant_match_pct < 100.0,
        "a divergent real capability decision (revoked) MUST be RED: {red:?}"
    );
}

/// REGION-PIN class observed through the real in-process kernel:
/// `enforce_region` at the write chokepoint. A home write → Ok (GREEN across
/// forms); a cross-region write → `ERegionViolation` (RED).
#[test]
fn region_pin_observed_via_in_process_kernel() {
    let native = vec![capture_native(
        "region-ker",
        InvariantClass::RegionPin,
        kernel_oracle::region_violation_effect(false),
    )];
    let wasm = vec![capture_wasm(
        "region-ker",
        InvariantClass::RegionPin,
        kernel_oracle::region_violation_effect(false),
    )];
    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed && verdict.invariant_match_pct == 100.0,
        "identical real region-pin decision must be GREEN: {verdict:?}"
    );

    // A cross-region (foreign) write → a different real decision → RED.
    let wasm_div = vec![capture_wasm(
        "region-ker",
        InvariantClass::RegionPin,
        kernel_oracle::region_violation_effect(true),
    )];
    let red = compare_effects(&native, &wasm_div);
    assert!(
        !red.passed && red.invariant_match_pct < 100.0,
        "a cross-region write (real ERegionViolation) MUST be RED: {red:?}"
    );
}

/// AUDIT class observed through the real in-process kernel: the `cap_audit`
/// channel carries the real `capability.invocation` event. The same number of
/// real verifies → the same real audit-event count (GREEN across forms); a
/// different number → a different count (RED).
#[test]
fn audit_observed_via_in_process_kernel() {
    let (adapter, mut rx) = kernel_oracle::cap_registry();
    let one = kernel_oracle::capability_invocation_audit_count(&adapter, &mut rx, 1);
    assert!(one >= 1, "sanity: a verify emits ≥1 audit event, got {one}");

    let native = vec![capture_native(
        "audit-ker",
        InvariantClass::Audit,
        effect_audit(AUDIT_KIND_CAPABILITY_INVOCATION, one),
    )];
    let wasm = vec![capture_wasm(
        "audit-ker",
        InvariantClass::Audit,
        effect_audit(AUDIT_KIND_CAPABILITY_INVOCATION, one),
    )];
    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed && verdict.invariant_match_pct == 100.0,
        "identical real audit-event count must be GREEN: {verdict:?}"
    );

    // A different number of real verifies → a different audit-event count → RED.
    let two = kernel_oracle::capability_invocation_audit_count(&adapter, &mut rx, 2);
    assert_ne!(two, one, "sanity: more verifies emit more audit events");
    let wasm_div = vec![capture_wasm(
        "audit-ker",
        InvariantClass::Audit,
        effect_audit(AUDIT_KIND_CAPABILITY_INVOCATION, two),
    )];
    let red = compare_effects(&native, &wasm_div);
    assert!(
        !red.passed && red.invariant_match_pct < 100.0,
        "a divergent real audit-event count MUST be RED: {red:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// §7b  Forged-consent negative control — kernel derives, not trusts (finding 2)
// ────────────────────────────────────────────────────────────────────────

/// Forged in-band consent is NON-INVARIANT: the native form carries a forged
/// `consent_envelope` (and a forged `intent`) that the WASM form CANNOT carry
/// through the WIT bridge, yet the kernel-side decision is identical across
/// forms — because the kernel DERIVES enforcement and ignores the in-band
/// field, and `normalize` excludes exactly the F3 set. This is the
/// kernel-oracle independence proof (D12): the dropped field cannot move the
/// verdict, so the oracle is not a mirror of the lossy bridge.
#[test]
fn forged_consent_is_non_invariant() {
    // The F3 allowlist must actually carry the consent field — the proof is
    // tied to the allowlist, not to a hardcoded expectation.
    assert!(
        F3_EXCLUDED_FIELDS.contains(&"consent_envelope"),
        "consent_envelope MUST be in F3_EXCLUDED_FIELDS (the forged-consent \
         control depends on it being excluded)"
    );
    assert!(
        F3_EXCLUDED_FIELDS.contains(&"intent"),
        "intent MUST be in F3_EXCLUDED_FIELDS"
    );

    // Native twin: forge an in-band intent the WASM bridge cannot carry.
    let mut native_frame = sample_identity_frame();
    native_frame.intent = IntentClass::HighPrivilege; // forged in-band intent
                                                      // WASM form: the bridge defaults intent to Readonly and cannot carry the
                                                      // native's forged value — yet it MUST compare equal after normalization.
    let mut wasm_frame = sample_identity_frame();
    wasm_frame.intent = IntentClass::Readonly;

    // Forged intent on native vs defaulted intent on wasm → identical
    // normalized frames (intent is F3-excluded) → identical invariant verdict.
    let native = vec![capture_native_frame_sequence(
        "forged-consent",
        vec![native_frame],
    )];
    let wasm = vec![capture_wasm_frame_sequence(
        "forged-consent",
        vec![wasm_frame],
    )];

    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed,
        "forged in-band consent/intent MUST NOT move the verdict (the kernel \
         derives and the field is F3-excluded): {verdict:?}"
    );
    assert_eq!(
        verdict.invariant_match_pct, 100.0,
        "the forged-consent control proves the field is non-invariant — \
         invariant tier stays 100%"
    );

    // Control-of-the-control: a REAL invariant divergence (logical_clock)
    // still flips RED, proving the oracle is not blind — it just correctly
    // ignores the non-invariant consent/intent field.
    let mut divergent_frame = sample_identity_frame();
    divergent_frame.intent = IntentClass::HighPrivilege;
    divergent_frame.logical_clock = sample_identity_frame().logical_clock + 1;
    let wasm_div = vec![capture_wasm_frame_sequence(
        "forged-consent",
        vec![divergent_frame],
    )];
    let red = compare_effects(&native, &wasm_div);
    assert!(
        !red.passed && red.invariant_match_pct < 100.0,
        "a real invariant divergence MUST still be RED even with forged \
         consent present: {red:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// §7c  Cosmetic tier — the 75% threshold actually bites (finding 7)
// ────────────────────────────────────────────────────────────────────────

/// The cosmetic tier is NOT vacuous. A multi-effect corpus where the
/// invariant tier is 100% but enough `spirit_pid` (cosmetic) values drift
/// drops the cosmetic tier BELOW 75% and flips the gate RED — proving the
/// threshold bites, not just rubber-stamps.
#[test]
fn cosmetic_threshold_bites_through_invariant_green() {
    let receipt = sample_halt_receipt();
    // Four halt effects, identical halt_ids (invariant 100%), but the WASM
    // form reports a different logical spirit_pid on TWO of them.
    let mut native = Vec::new();
    let mut wasm = Vec::new();
    for i in 0..4u32 {
        let r = HaltReceipt::new(
            HaltId::new(&format!("halt-{i}")).unwrap(),
            receipt.timestamp_ns,
            100,
            receipt.boot_nonce,
            receipt.frame_id,
        );
        native.push(capture_native("cos", InvariantClass::Halt, effect_halt(&r)));
        // Drift spirit_pid on half the effects (cosmetic divergence).
        let drifted_pid = if i >= 2 { 777 } else { 100 };
        let r_wasm = HaltReceipt::new(
            HaltId::new(&format!("halt-{i}")).unwrap(),
            receipt.timestamp_ns,
            drifted_pid,
            receipt.boot_nonce,
            receipt.frame_id,
        );
        wasm.push(capture_wasm(
            "cos",
            InvariantClass::Halt,
            effect_halt(&r_wasm),
        ));
    }

    let verdict = compare_effects(&native, &wasm);
    // Invariant tier is perfect (all halt_ids match)...
    assert_eq!(
        verdict.invariant_match_pct, 100.0,
        "invariant tier must be 100% (only cosmetic drifts): {verdict:?}"
    );
    // ...but cosmetic is 2/4 = 50% < 75% → the gate flips RED via the
    // cosmetic tier. This is the non-vacuous bite.
    assert!(
        verdict.cosmetic_match_pct < 75.0,
        "cosmetic tier must drop below 75%: {verdict:?}"
    );
    assert!(
        !verdict.passed,
        "cosmetic-only drift below the 75% threshold MUST flip the gate RED: \
         {verdict:?}"
    );

    // And exactly-at-75% (3/4) stays GREEN (>= threshold), proving the bound.
    let mut wasm3 = wasm.clone();
    // Fix one of the two drifted effects so 3/4 match → exactly 75%.
    if let EffectData::HaltReceipt { spirit_pid, .. } = &mut wasm3[2].data {
        *spirit_pid = 100;
    }
    let at75 = compare_effects(&native, &wasm3);
    assert!(
        at75.passed,
        "cosmetic at exactly 75% (3/4) MUST stay GREEN (>= threshold): {at75:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// §7d  Robust pairing — duplicate keys cannot cross-wire (finding 8)
// ────────────────────────────────────────────────────────────────────────

/// Two streams with DUPLICATE `(scenario, class)` keys but an EQUAL multiset
/// MUST pair correctly and stay GREEN. Without the content sub-key, stable
/// sort preserves per-form emission order (which differs), index-pairs
/// unequal values, and produces a false RED. The content-key sort makes the
/// pairing a bijection on the multiset.
#[test]
fn duplicate_key_pairing_is_robust() {
    let mk = |halt_id: &str| {
        effect_halt(&HaltReceipt::new(
            HaltId::new(halt_id).unwrap(),
            1,
            7,
            0,
            [0; 16],
        ))
    };
    // Native emits [a, b]; wasm emits the SAME multiset but in scrambled
    // order [b, a]. A sort by (scenario, class) alone leaves emission order
    // intact → cross-wire false RED. The content sub-key sorts both to [a,b].
    let native = vec![
        capture_native("dup", InvariantClass::Halt, mk("a")),
        capture_native("dup", InvariantClass::Halt, mk("b")),
    ];
    let wasm = vec![
        capture_wasm("dup", InvariantClass::Halt, mk("b")),
        capture_wasm("dup", InvariantClass::Halt, mk("a")),
    ];

    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed,
        "equal multiset with duplicate keys MUST be GREEN (no cross-wire): {verdict:?}"
    );
    assert_eq!(verdict.invariant_match_pct, 100.0);

    // And a genuinely unequal multiset is still caught (one extra 'c').
    let native_extra = vec![
        capture_native("dup", InvariantClass::Halt, mk("a")),
        capture_native("dup", InvariantClass::Halt, mk("b")),
        capture_native("dup", InvariantClass::Halt, mk("c")),
    ];
    let red = compare_effects(&native_extra, &wasm);
    assert!(
        !red.passed,
        "an unequal multiset MUST still be RED: {red:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// §7e  F3 tier-map honesty — allowlist pinned + behaviorally matched (finding 10)
// ────────────────────────────────────────────────────────────────────────

/// The F3 excluded set is pinned to exactly the four fields the WIT bridge
/// drops. Editing [`F3_EXCLUDED_FIELDS`] flips this test — the set is
/// grep-able and mutation-guarded.
#[test]
fn f3_allowlist_is_pinned_to_the_dropped_set() {
    assert_eq!(
        F3_EXCLUDED_FIELDS,
        &["intent", "consent_envelope", "intent_lineage", "scope"],
        "F3_EXCLUDED_FIELDS is the enumerated contract — editing it MUST be \
         a deliberate act that updates this assertion"
    );
}

/// `normalize` behaviorally drops EXACTLY the F3-excluded fields: frames
/// differing ONLY in an excluded field normalize equal, while frames
/// differing in a PRESERVED field normalize different. This ties the
/// allowlist const to actual normalization behavior.
#[test]
fn normalize_ignores_exactly_the_f3_excluded_fields() {
    // Excluded `intent`: two distinct values → same normalized frame.
    let mut a = sample_identity_frame();
    a.intent = IntentClass::Standard;
    let mut b = sample_identity_frame();
    b.intent = IntentClass::Readonly;
    assert_eq!(normalize(&a), normalize(&b), "intent is F3-excluded");

    // Excluded `scope` (TaskAssign sub-field): non-empty vs empty → same.
    let mut c = sample_identity_frame();
    if let FramePayload::TaskAssign(ta) = &mut c.payload {
        ta.scope.push(maos_domain::invariants::i1::Scope::FsRead {
            subtree: "/tmp".to_string(),
        });
    }
    let d = sample_identity_frame();
    assert_eq!(normalize(&c), normalize(&d), "scope is F3-excluded");
    // ...and the scope really is zeroed, not merely equal-by-luck.
    if let FramePayload::TaskAssign(ta) = &normalize(&c).payload {
        assert!(ta.scope.is_empty(), "normalize must zero the scope");
    }

    // Preserved `logical_clock`: a difference survives normalization.
    let mut e = sample_identity_frame();
    e.logical_clock += 1;
    assert_ne!(
        normalize(&sample_identity_frame()),
        normalize(&e),
        "logical_clock is preserved — its divergence MUST survive normalize"
    );
}

/// EVERY preserved `NormalizedFrame` field is invariant: diverging any one of
/// them flips the gate RED, and NONE of them is in the F3 excluded set. This
/// broadens the tier-map honesty guard beyond `logical_clock` alone.
#[test]
fn all_invariant_fields_are_guarded_against_demotion() {
    // No preserved field may silently appear in the excluded set.
    for field in [
        "frame_id",
        "timestamp_ns",
        "logical_clock",
        "from",
        "to",
        "kind",
        "payload",
        "auto_marker",
    ] {
        assert!(
            !F3_EXCLUDED_FIELDS.contains(&field),
            "{field} is invariant — it MUST NOT be in F3_EXCLUDED_FIELDS"
        );
    }

    let base = sample_identity_frame();

    // Diverge `logical_clock`.
    let mut lc = base.clone();
    lc.logical_clock += 1;
    assert_red_on_single_divergence(&base, lc, "logical_clock");

    // Diverge `kind`.
    let mut k = base.clone();
    k.kind = match base.kind {
        FrameKind::TaskAssign => FrameKind::TaskComplete,
        other => other,
    };
    assert_red_on_single_divergence(&base, k, "kind");

    // Diverge `from`.
    let mut f = base.clone();
    f.from = FrameAddress {
        spirit_id: "other-from".into(),
        host_id: None,
        role: None,
    };
    assert_red_on_single_divergence(&base, f, "from");

    // Diverge `to`.
    let mut t = base.clone();
    t.to = smallvec::smallvec![FrameAddress {
        spirit_id: "other-to".into(),
        host_id: None,
        role: None,
    }];
    assert_red_on_single_divergence(&base, t, "to");

    // Diverge `auto_marker`.
    let mut am = base.clone();
    am.auto_marker = match base.auto_marker {
        FrameOrigin::SpiritAuto => FrameOrigin::HumanAuthored,
        other => other,
    };
    assert_red_on_single_divergence(&base, am, "auto_marker");
}

/// Helper: two single-frame captures differing ONLY by `divergent` must be RED.
fn assert_red_on_single_divergence(base: &IacFrame, divergent: IacFrame, field: &str) {
    let native = vec![capture_native_frame_sequence("guard", vec![base.clone()])];
    let wasm = vec![capture_wasm_frame_sequence("guard", vec![divergent])];
    let verdict = compare_effects(&native, &wasm);
    assert!(
        !verdict.passed && verdict.invariant_match_pct < 100.0,
        "{field} is invariant — its divergence MUST be RED (not silently \
         demoted): {verdict:?}"
    );
}

/// Tier-map honesty (original guard, retained): `logical_clock` specifically
/// is invariant and MUST NOT be demoted into the excluded set.
#[test]
fn tier_demotion_causes_red() {
    assert!(
        !F3_EXCLUDED_FIELDS.contains(&"logical_clock"),
        "logical_clock MUST remain invariant — it is not in F3_EXCLUDED_FIELDS"
    );
    let native_frame = sample_identity_frame();
    let mut wasm_frame = sample_identity_frame();
    wasm_frame.logical_clock = native_frame.logical_clock + 1;
    let native = vec![capture_native_frame_sequence(
        "demotion",
        vec![native_frame],
    )];
    let wasm = vec![capture_wasm_frame_sequence("demotion", vec![wasm_frame])];
    let verdict = compare_effects(&native, &wasm);
    assert!(
        !verdict.passed,
        "logical_clock is invariant — its divergence MUST be RED: {verdict:?}"
    );
    assert!(verdict.invariant_match_pct < 100.0);
}

// ────────────────────────────────────────────────────────────────────────
// §7f  Anti-canned tripwire — broadened + boundary (finding 12)
// ────────────────────────────────────────────────────────────────────────

/// Anti-canned tripwire: under `equiv-fault-inject`, perturbing a captured
/// invariant-tier effect must move the verdict. Covers the frame-sequence
/// class (logical_clock, saturating — matching the real divergent transform).
#[cfg(feature = "equiv-fault-inject")]
#[test]
fn fault_injection_moves_the_number() {
    let frame = sample_identity_frame();

    let native_clean = vec![capture_native_frame_sequence("fault", vec![frame.clone()])];
    let wasm = vec![capture_wasm_frame_sequence("fault", vec![frame])];

    let clean_verdict = compare_effects(&native_clean, &wasm);
    assert!(
        clean_verdict.passed,
        "baseline (no fault) must be GREEN: {clean_verdict:?}"
    );

    // Inject AFTER capture, BEFORE comparison.
    let mut native_faulted = native_clean.clone();
    inject_invariant_fault(&mut native_faulted[0]);

    let faulted_verdict = compare_effects(&native_faulted, &wasm);
    assert!(
        !faulted_verdict.passed,
        "fault injection MUST move the verdict to RED: {faulted_verdict:?}"
    );
    assert_ne!(
        clean_verdict.invariant_match_pct, faulted_verdict.invariant_match_pct,
        "anti-canned: the comparator must respond to the perturbation, \
         not return a canned number"
    );
}

/// The anti-canned tripwire covers EVERY invariant class (not just
/// frame-sequence): perturbing a halt / capability / region / audit effect
/// must move the verdict. A comparator that only listened to frames would
/// pass the frame tripwire but die here.
#[cfg(feature = "equiv-fault-inject")]
#[test]
fn fault_injection_covers_every_invariant_class() {
    let receipt = sample_halt_receipt();

    // Halt.
    let n = capture_native("halt", InvariantClass::Halt, effect_halt(&receipt));
    let w = capture_wasm("halt", InvariantClass::Halt, effect_halt(&receipt));
    assert_fault_moves_verdict(n, w, "halt");

    // Capability denial.
    let n = capture_native(
        "cap",
        InvariantClass::CapabilityDenial,
        effect_capability_denial(&CapError::PolicyDenied, 1),
    );
    let w = capture_wasm(
        "cap",
        InvariantClass::CapabilityDenial,
        effect_capability_denial(&CapError::PolicyDenied, 1),
    );
    assert_fault_moves_verdict(n, w, "cap");

    // Region.
    let home = Region::canonicalize("eu").unwrap();
    let attempted = Region::canonicalize("us").unwrap();
    let n = capture_native(
        "region",
        InvariantClass::RegionPin,
        effect_region_violation(&attempted, &home),
    );
    let w = capture_wasm(
        "region",
        InvariantClass::RegionPin,
        effect_region_violation(&attempted, &home),
    );
    assert_fault_moves_verdict(n, w, "region");

    // Audit.
    let n = capture_native(
        "audit",
        InvariantClass::Audit,
        effect_audit(AUDIT_KIND_CAPABILITY_INVOCATION, 2),
    );
    let w = capture_wasm(
        "audit",
        InvariantClass::Audit,
        effect_audit(AUDIT_KIND_CAPABILITY_INVOCATION, 2),
    );
    assert_fault_moves_verdict(n, w, "audit");
}

#[cfg(feature = "equiv-fault-inject")]
fn assert_fault_moves_verdict(native: CapturedEffect, wasm: CapturedEffect, class: &str) {
    let clean = compare_effects(std::slice::from_ref(&native), std::slice::from_ref(&wasm));
    assert!(clean.passed, "[{class}] baseline must be GREEN: {clean:?}");
    let mut faulted = native.clone();
    inject_invariant_fault(&mut faulted);
    let red = compare_effects(std::slice::from_ref(&faulted), std::slice::from_ref(&wasm));
    assert!(
        !red.passed,
        "[{class}] fault injection MUST move the verdict to RED: {red:?}"
    );
    assert_ne!(
        clean.invariant_match_pct, red.invariant_match_pct,
        "[{class}] anti-canned: the comparator must respond to the perturbation"
    );
}

/// At the `u64::MAX` ceiling, the saturating divergent transform is a no-op
/// (MAX + 1 = MAX) — so identity and the real saturating-divergent behavior
/// are GENUINELY equivalent there (an honest non-divergence, not a bug). But
/// the comparator must still respond to a GUARANTEED-distinct perturbation at
/// the boundary, proving it is not canned where saturating cannot move the
/// number.
#[test]
fn saturating_diverge_at_u64_max_ceiling_is_honest() {
    let mut at_max = sample_identity_frame();
    at_max.logical_clock = u64::MAX;
    // Saturating +1 at MAX = MAX → genuinely equal → GREEN (honest).
    let mut saturated = at_max.clone();
    saturated.logical_clock = at_max.logical_clock.saturating_add(1);
    let native = vec![capture_native_frame_sequence("max", vec![at_max])];
    let wasm = vec![capture_wasm_frame_sequence("max", vec![saturated])];
    let verdict = compare_effects(&native, &wasm);
    assert!(
        verdict.passed,
        "at u64::MAX the saturating divergent transform is a no-op — the \
         forms are genuinely equivalent there (honest GREEN): {verdict:?}"
    );
}

#[cfg(feature = "equiv-fault-inject")]
#[test]
fn fault_injection_boundary_moves_under_distinct_perturbation() {
    let mut at_max = sample_identity_frame();
    at_max.logical_clock = u64::MAX;

    let native = vec![capture_native_frame_sequence(
        "max-fault",
        vec![at_max.clone()],
    )];
    let wasm = vec![capture_wasm_frame_sequence("max-fault", vec![at_max])];
    let clean = compare_effects(&native, &wasm);
    assert!(clean.passed, "baseline at MAX must be GREEN: {clean:?}");

    // A guaranteed-distinct perturbation at the boundary MUST still move the
    // number — the comparator is not canned even where saturating cannot help.
    let mut faulted = native.clone();
    inject_invariant_fault_distinct(&mut faulted[0]);
    // Sanity: the distinct injection actually changed the value away from MAX.
    if let EffectData::FrameSequence(frames) = &faulted[0].data {
        assert_ne!(
            frames[0].logical_clock,
            u64::MAX,
            "distinct perturbation must move the value off the MAX ceiling"
        );
    } else {
        panic!("expected FrameSequence after fault injection");
    }
    let red = compare_effects(&faulted, &wasm);
    assert!(
        !red.passed,
        "a distinct perturbation at u64::MAX MUST move the verdict to RED: {red:?}"
    );
    assert_ne!(clean.invariant_match_pct, red.invariant_match_pct);
}
