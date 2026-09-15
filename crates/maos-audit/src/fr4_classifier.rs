#![forbid(unsafe_code)]

//! Story 16-2 / D-16-2-G — the FR4 projection's view of rows that are not
//! calls.
//!
//! FR4 (the 100% mediation view) refuses a `capability.invocation` row with
//! no token — correctly, because an unmediated call is exactly what it exists
//! to catch. But the kernel writes MANY rows that are not calls at all: halt
//! rows, lifecycle transitions, telemetry, cost attribution. Before this
//! module every one of them red the whole `--spirit` view.
//!
//! The projection now CLASSIFIES before it validates, against an exact
//! inventory of the tree's tokenless writers:
//!
//! (a) a row whose KIND is never an external call by its `FrameKind`
//!     definition — `epistemic.halt`, `telemetry.event`, budget warnings,
//!     stalls, silent-failure suspects, admissions, governance, cost
//!     attribution — is a non-call kernel event;
//! (b) a `capability.invocation` row with no token is a non-call kernel
//!     event iff it matches, EXACTLY, a `NonCall` entry of the writer-shape
//!     table below — one entry per tokenless kind-7 writer site, carrying
//!     the intent (or a `:`-terminated prefix) and the payload keys/types
//!     that writer sets. A prefix match is refused (`lifecycle.bogus` is a
//!     call). A `Call` entry never exempts.
//!
//! Every other row IS a call, and a call with no token is an
//! `Fr4SchemaViolation` exactly as before — fail-closed: an unknown writer
//! is a call until someone classifies it. The inventory test
//! (`tests/fr4_classifier_16_2.rs`) re-derives the writer sites from source
//! and reds when one has no disposition, so "every kernel row is classified"
//! stays true after this commit.
//!
//! The table is permanent: Transparency Logs written before a future re-kind
//! keep their kind-7 rows forever (§15 R3).

use crate::AuditEntry;

/// A row's disposition under the FR4 projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fr4RowDisposition {
    /// An external capability call — must carry a token.
    Call,
    /// A kernel event that is not a call. Rendered in the plain table
    /// (which has no token column), omitted from the NDJSON feed (whose
    /// per-line schema requires the token).
    NonCallKernelEvent,
}

/// Kinds that are never an external call by their `FrameKind` definition
/// (set (a)). `kind_to_string` must name each symmetrically.
pub const NON_CALL_KINDS: &[&str] = &[
    "epistemic.halt",         // 3
    "telemetry.event",        // 4
    "budget.warning",         // 12
    "budget.exceeded",        // 13
    "task.stalled",           // 15
    "silent.failure.suspect", // 16
    "spirit.admitted",        // 19
    "governance.event",       // 28
    "cost.attribution",       // 29
];

/// The payload value type a writer-shape entry pins for one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    /// Any JSON string.
    Str,
    /// This exact JSON string.
    StrEq(&'static str),
    /// Any JSON number.
    Num,
    /// JSON number or null (an `Option<u*_>` in the writer).
    NumOrNull,
    /// A JSON array of numbers (`[u8; N]` serializes as one) or null.
    NumArrayOrNull,
    /// JSON boolean.
    Bool,
    /// A JSON number equal to the ROW's `spirit_pid` — the writer echoes the
    /// row's own pid in the payload.
    SpiritPidEqualsRow,
}

/// The intent grammar of one writer site: an exact literal, or a fixed
/// prefix ending in `:` (templated intents such as `schedule.fire:<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterIntent {
    Exact(&'static str),
    Prefix(&'static str),
}

impl WriterIntent {
    /// Does the writer's intent grammar accept this row's intent?
    pub fn matches(&self, intent: &str) -> bool {
        match self {
            WriterIntent::Exact(exact) => intent == *exact,
            WriterIntent::Prefix(prefix) => {
                intent.len() > prefix.len() && intent.starts_with(prefix)
            }
        }
    }
}

/// The disposition of one tokenless kind-7 writer site.
#[derive(Debug, Clone, Copy)]
pub struct WriterShapeEntry {
    /// The DOORBELL key: (file, enclosing fn, ordinal of this writer call
    /// within that fn). Keyed by call site, never by the intent literal —
    /// one fn can hold two writers with the same intent shape.
    pub site: (&'static str, &'static str, u32),
    pub intent: WriterIntent,
    /// `None` for `Call` entries (a call is never exempt; the payload shape
    /// is irrelevant).
    pub shape: Option<&'static [(&'static str, PayloadType)]>,
}

impl WriterShapeEntry {
    /// A `Call` disposition — deliberately NOT exempt: if the row ever lands
    /// at a Spirit's pid it is a genuine FR4 finding.
    pub const fn call(file: &'static str, f: &'static str, ordinal: u32) -> Self {
        Self {
            site: (file, f, ordinal),
            intent: WriterIntent::Exact(""),
            shape: None,
        }
    }
}

/// THE writer-shape table — every tokenless kind-7 (and variable-kind)
/// `insert_frame_event*` site in `crates/maos-kernel-core/src`,
/// `crates/maos-bin/src` and `crates/maos-iac/src`, each measured at T0
/// against the tree (not inferred from this list).
///
/// The eight lifecycle shapes are NOT uniform: `start`/`pause`/`resume`/
/// `unload` carry `spirit_pid`, not `spirit_id` (validation round 1).
pub const WRITER_SHAPES: &[WriterShapeEntry] = &[
    // ── maos-kernel-core — NonCall ─────────────────────────────────────────
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "load", 0),
        intent: WriterIntent::Exact("lifecycle.admit"),
        shape: Some(&[("spirit_id", PayloadType::Str)]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "load", 1),
        intent: WriterIntent::Exact("lifecycle.load"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Load")),
            ("spirit_id", PayloadType::Str),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "start", 0),
        intent: WriterIntent::Exact("lifecycle.start"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Start")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "pause", 0),
        intent: WriterIntent::Exact("lifecycle.pause"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Pause")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "resume", 0),
        intent: WriterIntent::Exact("lifecycle.resume"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Resume")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "unload", 0),
        intent: WriterIntent::Exact("lifecycle.unload"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Unload")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "journal_lifecycle", 0),
        intent: WriterIntent::Exact("lifecycle.journal"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::Str),
            ("spirit_id", PayloadType::Str),
        ]),
    },
    WriterShapeEntry {
        site: ("crash_detector.rs", "handle_crash", 1),
        intent: WriterIntent::Exact("lifecycle.crash"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Crash")),
            ("spirit_id", PayloadType::Str),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
            ("cause", PayloadType::Str),
        ]),
    },
    WriterShapeEntry {
        // The payload is a plain `format!` STRING, not JSON.
        site: ("self_telemetry.rs", "self_telemetry", 0),
        intent: WriterIntent::Exact("telemetry.self"),
        shape: Some(&[]),
    },
    WriterShapeEntry {
        // `iac::payload::ScheduleFireRecord`: spirit_id, schedule_id,
        // fired_at_ns, compliance_claim_ref (Option<[u8;32]> → array|null),
        // side_effect_token_id (TokenId([u8;16]) → array),
        // principal_revocability (bool).
        site: ("schedule_watchdog.rs", "check_and_fire", 0),
        intent: WriterIntent::Prefix("schedule.fire:"),
        shape: Some(&[
            ("spirit_id", PayloadType::Str),
            ("schedule_id", PayloadType::Str),
            ("fired_at_ns", PayloadType::Num),
            ("compliance_claim_ref", PayloadType::NumArrayOrNull),
            ("side_effect_token_id", PayloadType::NumArrayOrNull),
            ("principal_revocability", PayloadType::Bool),
        ]),
    },
    WriterShapeEntry {
        // KIND IS A VARIABLE — constrained to BudgetWarning/BudgetExceeded
        // (both in set (a)); the site demands an explicit entry regardless
        // (§15 R3: the doorbell's unit is the call site).
        site: ("hook_dispatch.rs", "emit_budget_frame", 0),
        intent: WriterIntent::Prefix("hook.budget."),
        shape: Some(&[
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
            ("hook_name", PayloadType::Str),
            ("wall_ns", PayloadType::Num),
            ("cap_seconds", PayloadType::Num),
            ("ratio_breached", PayloadType::Num),
        ]),
    },
    WriterShapeEntry {
        site: ("applier.rs", "apply_crl", 1),
        intent: WriterIntent::Exact("spirit.quarantine_requested"),
        shape: Some(&[
            ("spirit_id", PayloadType::Str),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
            ("quarantine_requested", PayloadType::Bool),
        ]),
    },
    WriterShapeEntry {
        site: ("upgrade.rs", "upgrade", 0),
        intent: WriterIntent::Exact("spirit.upgrade"),
        shape: Some(&[
            ("spirit_id", PayloadType::Str),
            ("predecessor_version", PayloadType::Str),
            ("successor_version", PayloadType::Str),
            ("policy", PayloadType::Str),
            ("outcome", PayloadType::Str),
            ("latency_ns", PayloadType::Num),
            ("halt_receipts_produced", PayloadType::Num),
        ]),
    },
    WriterShapeEntry {
        // `cli_wrapper/runtime.rs` — `insert_frame_event_with_sender`,
        // token `None`.
        site: ("runtime.rs", "wait_and_finalize", 0),
        intent: WriterIntent::Exact("cli.subprocess.exit"),
        shape: Some(&[
            ("event", PayloadType::StrEq("cli_subprocess_exit")),
            ("cli_child_pid", PayloadType::NumOrNull),
            ("exit_cause", PayloadType::Str),
            ("is_crash", PayloadType::Bool),
        ]),
    },
    // ── maos-bin — NonCall ─────────────────────────────────────────────────
    WriterShapeEntry {
        // The smoke-orchestrator-fanout arm — different keys from the
        // runtime writer above; both entries exist because the doorbell
        // keys the CALL SITE.
        site: ("main.rs", "smoke_orchestrator_fanout_6_2", 3),
        intent: WriterIntent::Exact("cli.subprocess.exit"),
        shape: Some(&[
            ("cli", PayloadType::Str),
            ("exit_code", PayloadType::Num),
            ("bytes", PayloadType::Num),
            ("duration_ms", PayloadType::Num),
        ]),
    },
    // ── maos-iac — Call (deliberately NOT exempt) ──────────────────────────
    WriterShapeEntry::call("adapter.rs", "deliver_typed", 0), // `_with_id`, variable `tl_kind`
    WriterShapeEntry::call("drr_scheduler.rs", "flush_batch", 0), // `_with_id`, variable `sub.tl_kind`
    WriterShapeEntry::call("log_recall.rs", "journal_cross_wall_recall", 0),
    WriterShapeEntry::call("log_recall.rs", "recall", 0),
    WriterShapeEntry::call("log_recall.rs", "fetch", 0),
    WriterShapeEntry::call("distillate.rs", "write_distillate", 0),
];

/// Classify one row for the FR4 projection: call, or non-call kernel event.
pub fn classify_fr4_row(entry: &AuditEntry) -> Fr4RowDisposition {
    // (a) kinds that are never an external call by definition.
    if NON_CALL_KINDS.contains(&entry.kind.as_str()) {
        return Fr4RowDisposition::NonCallKernelEvent;
    }
    // Only kind 7 reaches the writer-shape exemption.
    if entry.kind != "capability.invocation" {
        return Fr4RowDisposition::Call;
    }
    if entry.capability_token_hex.is_some() {
        return Fr4RowDisposition::Call;
    }
    for writer in WRITER_SHAPES {
        let Some(shape) = writer.shape else {
            continue; // Call entries never exempt
        };
        if !writer.intent.matches(&entry.intent) {
            continue;
        }
        if payload_matches(shape, &entry.payload, entry.spirit_pid) {
            return Fr4RowDisposition::NonCallKernelEvent;
        }
    }
    Fr4RowDisposition::Call
}

/// EXACT payload match: parses as a JSON object, every pinned key present
/// with the pinned type, and NO unknown keys — the payload the writer sets,
/// nothing else. A shape mismatch means the writer changed or the row is
/// forged: both are calls (fail-closed).
fn payload_matches(
    shape: &'static [(&'static str, PayloadType)],
    payload: &str,
    row_spirit_pid: u32,
) -> bool {
    // An empty shape pins a NON-JSON payload (`telemetry.self` writes a
    // plain string; an empty payload matches nothing here).
    if shape.is_empty() {
        if payload.is_empty() {
            return false;
        }
        return serde_json::from_str::<serde_json::Value>(payload).is_err();
    }
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(payload)
    else {
        return false;
    };
    if map.len() != shape.len() {
        return false;
    }
    for (key, ty) in shape {
        let Some(value) = map.get(*key) else {
            return false;
        };
        let ok = match ty {
            PayloadType::Str => value.is_string(),
            PayloadType::StrEq(expected) => value.as_str() == Some(*expected),
            PayloadType::Num => value.is_number(),
            PayloadType::NumOrNull => value.is_number() || value.is_null(),
            PayloadType::NumArrayOrNull => {
                value.is_null()
                    || value
                        .as_array()
                        .is_some_and(|a| a.iter().all(serde_json::Value::is_number))
            }
            PayloadType::Bool => value.is_boolean(),
            PayloadType::SpiritPidEqualsRow => value.as_u64() == Some(row_spirit_pid as u64),
        };
        if !ok {
            return false;
        }
    }
    true
}
