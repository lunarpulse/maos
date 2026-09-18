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
//!     stalls, silent-failure suspects, admissions, Worker subprocess
//!     output, governance, cost attribution, identity assertions — is a
//!     non-call kernel event, token-bearing or not (16-3 / D-16-3-J (1));
//! (b) a row that matches, EXACTLY, a `NonCall` entry of the writer-shape
//!     table below is a non-call kernel event. 16-2 measured one entry per
//!     tokenless kind-7 writer site, carrying the intent (or a
//!     `:`-terminated prefix) and the payload keys/types that writer sets.
//!     16-3 / D-16-3-J (2) adds the kind-1 (`task.complete`) FR50
//!     dispositions: every entry now pins the writer's KIND discriminator
//!     and TOKEN column (`kind: 7, token: Absent` stays the default, so
//!     the 16-2 entries are unchanged), and an entry matches only its own
//!     kind and the token state its writer permits. A kind-1
//!     `task.orphaned` row may be token-bearing or tokenless because the task
//!     record now preserves host-grant token absence; the payload's
//!     `in_flight_tokens` shape must agree with that state. A prefix match is
//!     refused (`lifecycle.bogus` is a call). A `Call` entry never exempts.
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
    "cli.subprocess.output",  // 21 — a Worker's own subprocess output rows land at its pid (16-3)
    "governance.event",       // 28
    "cost.attribution",       // 29
    "identity.asserted", // 30 — tokenless, minted at the Worker's real pid under enterprise posture (16-3)
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
    /// A JSON string or null (`Option<String>` in the writer) — a crash
    /// cause's `stderr_tail` (16-3 / D-16-3-J (3)).
    StrOrNull,
    /// A JSON array whose every element is itself an array of exactly 16
    /// numbers — `Vec<TokenId>`, where `TokenId([u8; 16])` serializes as a
    /// 16-number array (`i1.rs`). It must be empty when the row token is absent
    /// and non-empty when the row token is present. A flat number array or an
    /// element of any other length is refused.
    NumArrayArray,
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

/// Which token column a writer sets — the second dimension of a
/// writer-shape entry (16-3 / D-16-3-J (2)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenColumn {
    /// The writer passes `None` — the row's token column is NULL.
    Absent,
    /// The writer passes a token — the row's token column is populated.
    Present,
    /// The writer preserves an optional token from its input record.
    Optional,
}

/// The disposition of one writer site, pinned to its KIND discriminator and
/// TOKEN column (16-3 / D-16-3-J (2)).
#[derive(Debug, Clone, Copy)]
pub struct WriterShapeEntry {
    /// The DOORBELL key: (file, enclosing fn, ordinal of this writer call
    /// within that fn). Keyed by call site, never by the intent literal —
    /// one fn can hold two writers with the same intent shape.
    pub site: (&'static str, &'static str, u32),
    /// The `FrameKind` discriminator the writer passes. `7`
    /// (`capability.invocation`) is the default — those entries are
    /// evaluated after the kind-7 gate in [`classify_fr4_row`]; entries
    /// with any other kind (kind 1 today) are evaluated before it.
    pub kind: u8,
    /// The token column the writer sets. Fixed-token entries match only their
    /// own presence; optional-token entries also pin payload state to token
    /// presence.
    pub token: TokenColumn,
    /// The intent grammar of the site: an exact literal, or a fixed prefix
    /// ending in `:`.
    pub intent: WriterIntent,
    /// `None` for `Call` entries (a call is never exempt; the payload shape
    /// is irrelevant).
    pub shape: Option<&'static [(&'static str, PayloadType)]>,
}

impl WriterShapeEntry {
    /// A `Call` disposition — deliberately NOT exempt: if the row ever lands
    /// at a Spirit's pid it is a genuine FR4 finding. Defaults to the
    /// kind-7 tokenless writer shape the 16-2 table was measured against.
    pub const fn call(file: &'static str, f: &'static str, ordinal: u32) -> Self {
        Self {
            site: (file, f, ordinal),
            kind: 7,
            token: TokenColumn::Absent,
            intent: WriterIntent::Exact(""),
            shape: None,
        }
    }
}

/// THE writer-shape table — every `insert_frame_event*` writer site the
/// doorbell demands a disposition for: the tokenless kind-7 (and
/// variable-kind) sites (16-2), plus every kind-1 `TaskComplete` site,
/// token-bearing or not (16-3 / D-16-3-J (4)) — in
/// `crates/maos-kernel-core/src`, `crates/maos-bin/src` and
/// `crates/maos-iac/src`, each measured at T0 against the tree (not
/// inferred from this list).
///
/// The eight lifecycle shapes are NOT uniform: `start`/`pause`/`resume`/
/// `unload` carry `spirit_pid`, not `spirit_id` (validation round 1).
pub const WRITER_SHAPES: &[WriterShapeEntry] = &[
    // ── maos-kernel-core — NonCall ─────────────────────────────────────────
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "load", 0),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("lifecycle.admit"),
        shape: Some(&[("spirit_id", PayloadType::Str)]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "load", 1),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("lifecycle.load"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Load")),
            ("spirit_id", PayloadType::Str),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "start", 0),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("lifecycle.start"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Start")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "pause", 0),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("lifecycle.pause"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Pause")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "resume", 0),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("lifecycle.resume"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Resume")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "unload", 0),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("lifecycle.unload"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::StrEq("Unload")),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
        ]),
    },
    WriterShapeEntry {
        site: ("scheduler_loop.rs", "journal_lifecycle", 0),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("lifecycle.journal"),
        shape: Some(&[
            ("lifecycle_event", PayloadType::Str),
            ("spirit_id", PayloadType::Str),
        ]),
    },
    WriterShapeEntry {
        site: ("verb_resolver.rs", "resolve_verb", 0),
        kind: 2,
        token: TokenColumn::Absent,
        intent: WriterIntent::Prefix("lifecycle."),
        shape: Some(&[
            ("director", PayloadType::Str),
            ("spirit_id", PayloadType::Str),
            ("verb", PayloadType::Str),
        ]),
    },
    WriterShapeEntry {
        // Story 16-5: task IDs remain in the payload as 16-byte arrays; the
        // capability-token column stays NULL rather than padding them into
        // false 32-byte capability evidence.
        site: ("crash_detector.rs", "handle_crash", 0),
        kind: 1,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("task.orphaned"),
        shape: Some(&[
            ("task_id", PayloadType::Str),
            ("originator_spirit_id", PayloadType::Str),
            ("exit_signal", PayloadType::NumOrNull),
            ("exit_code", PayloadType::NumOrNull),
            ("stderr_tail", PayloadType::StrOrNull),
            ("cause", PayloadType::Str),
            ("in_flight_tokens", PayloadType::NumArrayArray),
            ("disposition", PayloadType::Str),
        ]),
    },
    WriterShapeEntry {
        site: ("crash_detector.rs", "handle_crash", 1),
        kind: 7,
        token: TokenColumn::Absent,
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
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("telemetry.self"),
        shape: Some(&[]),
    },
    WriterShapeEntry {
        // `iac::payload::ScheduleFireRecord`: spirit_id, schedule_id,
        // fired_at_ns, compliance_claim_ref (Option<[u8;32]> → array|null),
        // side_effect_token_id (TokenId([u8;16]) → array),
        // principal_revocability (bool).
        site: ("schedule_watchdog.rs", "check_and_fire", 0),
        kind: 7,
        token: TokenColumn::Absent,
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
        kind: 7,
        token: TokenColumn::Absent,
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
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("spirit.quarantine_requested"),
        shape: Some(&[
            ("spirit_id", PayloadType::Str),
            ("spirit_pid", PayloadType::SpiritPidEqualsRow),
            ("quarantine_requested", PayloadType::Bool),
        ]),
    },
    WriterShapeEntry {
        site: ("upgrade.rs", "upgrade", 0),
        kind: 7,
        token: TokenColumn::Absent,
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
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("cli.subprocess.exit"),
        shape: Some(&[
            ("event", PayloadType::StrEq("cli_subprocess_exit")),
            ("cli_child_pid", PayloadType::NumOrNull),
            ("exit_cause", PayloadType::Str),
            ("is_crash", PayloadType::Bool),
        ]),
    },
    // ── maos-kernel-core — NonCall ────────────────────────────────────────
    // AC2(b) / D-16-3-J (4): the SECOND `task.orphaned` writer
    // (`halt/resolver.rs`) writes structured JSON (`{"halt_id":…,
    // "disposition":"accepted_halt"}`) at pid 0 with no token — exactly
    // shape-matched, so the row is an honest kernel event, never a
    // mediated `Call`. A shape drift still fails closed to `Call`.
    WriterShapeEntry {
        site: ("resolver.rs", "emit_task_orphaned", 0),
        kind: 1,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("task.orphaned"),
        shape: Some(&[
            ("halt_id", PayloadType::Str),
            ("disposition", PayloadType::StrEq("accepted_halt")),
        ]),
    },
    // ── maos-bin — NonCall ─────────────────────────────────────────────────
    WriterShapeEntry {
        // The smoke-orchestrator-fanout arm — different keys from the
        // runtime writer above; both entries exist because the doorbell
        // keys the CALL SITE.
        site: ("main.rs", "smoke_orchestrator_fanout_6_2", 3),
        kind: 7,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("cli.subprocess.exit"),
        shape: Some(&[
            ("cli", PayloadType::Str),
            ("exit_code", PayloadType::Num),
            ("bytes", PayloadType::Num),
            ("duration_ms", PayloadType::Num),
        ]),
    },
    // ── maos-bin — Call (deliberately NOT exempt) ──────────────────────────
    // D-16-3-J (4): `smoke-distillate-source` seeds a raw source frame for
    // the smoke distillate write — a smoke seed, never a production row.
    WriterShapeEntry::call("main.rs", "smoke_orchestrator_fanout_6_2", 0),
    // ── maos-iac — NonCall (kind-1 FR50 dispositions, D-16-3-J (4)) ────────
    WriterShapeEntry {
        site: ("adapter.rs", "emit_task_complete_nack", 0),
        kind: 1,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("task.nacked"),
        shape: Some(&[
            ("task_id", PayloadType::Str),
            ("originator_spirit_id", PayloadType::Str),
            ("capability_token", PayloadType::NumArrayOrNull),
        ]),
    },
    WriterShapeEntry {
        site: ("adapter.rs", "emit_task_complete_escalated", 0),
        kind: 1,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("task.escalated"),
        shape: Some(&[
            ("task_id", PayloadType::Str),
            ("originator_spirit_id", PayloadType::Str),
            ("capability_token", PayloadType::NumArrayOrNull),
        ]),
    },
    WriterShapeEntry {
        site: ("adapter.rs", "reassign_task_to", 0),
        kind: 1,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("task.reassigned"),
        shape: Some(&[
            ("task_id", PayloadType::Str),
            ("originator_spirit_id", PayloadType::Str),
            ("capability_token", PayloadType::NumArrayOrNull),
            ("replica_spirit_id", PayloadType::Str),
        ]),
    },
    WriterShapeEntry {
        // Story 9.2's redaction marker — the distillate frame id as a hex
        // STRING (`format_frame_id_hex`).
        site: (
            "transparency_log.rs",
            "insert_distillate_redaction_marker",
            0,
        ),
        kind: 1,
        token: TokenColumn::Absent,
        intent: WriterIntent::Exact("distillate.redacted"),
        shape: Some(&[
            ("principal_id", PayloadType::Str),
            ("redacted_distillate_frame_id", PayloadType::Str),
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
    let token_present = entry.capability_token_hex.is_some();
    // (b) kind-dispositioned writer shapes. Entries whose kind is NOT 7 are
    // evaluated BEFORE the kind-7 gate: their rows are not
    // `capability.invocation` rows, so the gate would end them as `Call`
    // before the shape table ever saw them (D-16-3-J (2)). An entry matches
    // only its own kind — compared through `kind_from_string`, the inverse
    // of the same `kind_to_string` table the read side renders row kinds
    // with — and its own token column.
    for writer in WRITER_SHAPES.iter().filter(|w| w.kind != 7) {
        if crate::kind_from_string(&entry.kind) != Some(i64::from(writer.kind)) {
            continue;
        }
        if !token_column_matches(writer.token, token_present) {
            continue;
        }
        let Some(shape) = writer.shape else {
            continue; // Call entries never exempt
        };
        if writer.intent.matches(&entry.intent)
            && payload_matches(shape, &entry.payload, entry.spirit_pid, token_present)
        {
            return Fr4RowDisposition::NonCallKernelEvent;
        }
    }
    // Only kind 7 reaches the writer-shape exemption.
    if entry.kind != "capability.invocation" {
        return Fr4RowDisposition::Call;
    }
    if token_present {
        return Fr4RowDisposition::Call;
    }
    for writer in WRITER_SHAPES.iter().filter(|w| w.kind == 7) {
        if !token_column_matches(writer.token, token_present) {
            continue;
        }
        let Some(shape) = writer.shape else {
            continue; // Call entries never exempt
        };
        if writer.intent.matches(&entry.intent)
            && payload_matches(
                shape,
                &entry.payload,
                entry.spirit_pid,
                entry.capability_token_hex.is_some(),
            )
        {
            return Fr4RowDisposition::NonCallKernelEvent;
        }
    }
    Fr4RowDisposition::Call
}

/// Does the entry's pinned token column accept this row's token presence?
/// An entry matches only rows whose token presence equals its own
/// (D-16-3-J (2)).
fn token_column_matches(entry_token: TokenColumn, row_token_present: bool) -> bool {
    match entry_token {
        TokenColumn::Absent => !row_token_present,
        TokenColumn::Present => row_token_present,
        TokenColumn::Optional => true,
    }
}

/// EXACT payload match: parses as a JSON object, every pinned key present
/// with the pinned type, and NO unknown keys — the payload the writer sets,
/// nothing else. A shape mismatch means the writer changed or the row is
/// forged: both are calls (fail-closed).
fn payload_matches(
    shape: &'static [(&'static str, PayloadType)],
    payload: &str,
    row_spirit_pid: u32,
    _row_token_present: bool,
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
            PayloadType::StrOrNull => value.is_string() || value.is_null(),
            PayloadType::NumArrayArray => value.as_array().is_some_and(|outer| {
                outer.iter().all(|element| {
                    element.as_array().is_some_and(|inner| {
                        inner.len() == 16 && inner.iter().all(serde_json::Value::is_number)
                    })
                })
            }),
            PayloadType::Bool => value.is_boolean(),
            PayloadType::SpiritPidEqualsRow => value.as_u64() == Some(row_spirit_pid as u64),
        };
        if !ok {
            return false;
        }
    }
    true
}
