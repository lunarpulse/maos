#![forbid(unsafe_code)]

//! Story 16-3 / AC5 (D-16-3-J) — FR4 classification at a WORKER's pid.
//!
//! Once rows land at the Worker's pid, two facts decide whether
//! `maos audit query --spirit worker --format ndjson` exits 0 or dies on a
//! non-call row fed as a call: the Worker's own `cli.subprocess.output`
//! rows (kind 21, tokenless, `runtime.rs:607`) and the crash detector's
//! kind-1 `task.orphaned` rows (`crash_detector.rs:161-173`). The latter
//! preserve either a minted capability token or honest host-grant absence.
//! Every vector builds the row the REAL writer would have written — the
//! measured intent, token column and payload of the production site —
//! never the classifier's own table read back.

use std::path::Path;

use maos_audit::fr4_classifier::{classify_fr4_row, Fr4RowDisposition};
use maos_audit::{query, AuditEntry, AuditFilter};
use rusqlite::Connection;

/// The crash detector's measured `task.orphaned` payload. The 16-byte task
/// token stays in the payload and is never padded into the capability column.
const ORPHAN_PAYLOAD: &str = r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","exit_signal":9,"exit_code":null,"stderr_tail":null,"cause":"fault.signaled","in_flight_tokens":[[0,0,0,0,0,0,0,0,0,0,0,0,0,1]],"disposition":"nack"}"#;

const TOKENLESS_ORPHAN_PAYLOAD: &str = r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","exit_signal":9,"exit_code":null,"stderr_tail":null,"cause":"fault.signaled","in_flight_tokens":[],"disposition":"nack"}"#;

fn entry(kind: &str, intent: &str, pid: u32, token: Option<&str>, payload: &str) -> AuditEntry {
    AuditEntry {
        frame_id_hex: format!("{:032x}", pid),
        timestamp_ns: 1,
        spirit_pid: pid,
        boot_nonce: 7,
        capability_token_hex: token.map(|t| t.to_string()),
        kind: kind.to_string(),
        intent: intent.to_string(),
        payload: payload.to_string(),
        redaction: None,
    }
}

fn tokenless(kind: &str, intent: &str, pid: u32, payload: &str) -> AuditEntry {
    entry(kind, intent, pid, None, payload)
}

// ─────────────────────────────────────────────────────────────────────────────
// Kind 21 — the Worker's own subprocess output is never a call
// ─────────────────────────────────────────────────────────────────────────────

/// A kind-21 row is a non-call by its `FrameKind`, token-bearing or not
/// (D-16-3-J (1)): one row per captured output line, written tokenless at
/// the Worker's pid by `runtime.rs:607`.
#[test]
fn kind21_subprocess_output_is_a_non_call_token_or_not() {
    let token = "a".repeat(64);
    for tok in [None, Some(token.as_str())] {
        let row = entry(
            "cli.subprocess.output",
            "cli.subprocess.output",
            4242,
            tok,
            r#"{"cli":"codex","stream":"stdout","line":"worker alive","line_no":1,"child_pid":4243,"intent_lineage":"lin-1"}"#,
        );
        assert_eq!(
            classify_fr4_row(&row),
            Fr4RowDisposition::NonCallKernelEvent,
            "kind 21 must be a non-call (token present: {})",
            tok.is_some()
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Kind 1 — task.orphaned: task IDs never masquerade as capability tokens
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn padded_task_id_in_capability_column_fails_closed() {
    let row = entry(
        "task.complete",
        "task.orphaned",
        4242,
        Some("1111111111111111111111111111111100000000000000000000000000000000"),
        ORPHAN_PAYLOAD,
    );
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
}

#[test]
fn tokenless_host_grant_task_orphaned_is_a_non_call() {
    let row = tokenless(
        "task.complete",
        "task.orphaned",
        4242,
        TOKENLESS_ORPHAN_PAYLOAD,
    );
    assert_eq!(
        classify_fr4_row(&row),
        Fr4RowDisposition::NonCallKernelEvent
    );
}

/// A kind-7 row with the orphan intent and the orphan payload does not
/// reach the kind-1 entry — an entry matches only its OWN kind (D-16-3-J
/// (2)); the kind-7 exemption table has no `task.orphaned` entry.
#[test]
fn kind7_row_with_the_orphan_intent_stays_a_call() {
    let row = tokenless(
        "capability.invocation",
        "task.orphaned",
        4242,
        ORPHAN_PAYLOAD,
    );
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
}

/// `in_flight_tokens` is pinned to an array of 16-number arrays. A flat
/// number array or a 15-number element is refused; an empty array is valid
/// when the crashed task used host-grant authority.
#[test]
fn broken_in_flight_tokens_fail_closed() {
    for (tokens, label) in [
        (r#"[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]"#, "flat array"),
        (r#"[[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]]"#, "15-number element"),
    ] {
        let payload = ORPHAN_PAYLOAD.replace("[[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]]", tokens);
        let row = tokenless("task.complete", "task.orphaned", 4242, &payload);
        assert_eq!(
            classify_fr4_row(&row),
            Fr4RowDisposition::Call,
            "{label}: in_flight_tokens must be an array of 16-number arrays"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Kind 1 — the tokenless FR50 dispositions at pid 0
// ─────────────────────────────────────────────────────────────────────────────

/// `task.nacked` / `task.escalated` / `task.reassigned` /
/// `distillate.redacted` classify from their measured shapes
/// (`adapter.rs:246/268/292`, `transparency_log.rs:962`): tokenless kind-1
/// rows at pid 0 whose payloads pin `capability_token` as a
/// 16-number-array-or-null.
#[test]
fn tokenless_kind1_dispositions_classify_from_their_measured_payloads() {
    let rows: [(&str, &str); 4] = [
        (
            "task.nacked",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","capability_token":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2]}"#,
        ),
        (
            "task.escalated",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","capability_token":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3]}"#,
        ),
        (
            "task.reassigned",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","capability_token":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4],"replica_spirit_id":"butler-replica"}"#,
        ),
        (
            "distillate.redacted",
            r#"{"principal_id":"operator","redacted_distillate_frame_id":"01ab01ab01ab01ab01ab01ab01ab01ab"}"#,
        ),
    ];
    for (intent, payload) in rows {
        let row = tokenless("task.complete", intent, 0, payload);
        assert_eq!(
            classify_fr4_row(&row),
            Fr4RowDisposition::NonCallKernelEvent,
            "{intent}: the measured kind-1 disposition must classify as a non-call"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Kind 30 — the Worker's identity assertion is never a call
// ─────────────────────────────────────────────────────────────────────────────

/// The Worker's tokenless `identity.asserted` row (kind 30) at its real
/// pid: under an enterprise posture the Worker mints its own identity at
/// ITS pid (`persist_identity_asserted(spirit_pid, …)` — token column
/// NULL, raw kind-30 row), and an identity assertion is not a call
/// (D-16-3-J (1)).
#[test]
fn tokenless_kind30_identity_asserted_is_a_non_call() {
    let row = tokenless(
        "identity.asserted",
        "identity.asserted",
        4242,
        r#"{"subject":"worker-1","issuer":"https://idp.example","capability_key":"key-1","decision_time_ns":7}"#,
    );
    assert_eq!(
        classify_fr4_row(&row),
        Fr4RowDisposition::NonCallKernelEvent
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Regression guard — a REAL kind-15 row read back through maos_audit::query
// ─────────────────────────────────────────────────────────────────────────────

/// Same minimal Transparency-Log schema `identity_asserted_kind_test` pins —
/// single-sourced shape, no drift.
const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id BLOB NOT NULL PRIMARY KEY,
    timestamp_ns INTEGER NOT NULL,
    spirit_pid INTEGER NOT NULL,
    boot_nonce INTEGER NOT NULL,
    capability_token BLOB,
    kind INTEGER NOT NULL,
    intent TEXT NOT NULL,
    payload_redacted BLOB NOT NULL,
    origin INTEGER NOT NULL
);
";

/// END-TO-END guard: the scheduler's `task.stalled` row (kind 15,
/// tokenless, at the Worker's pid) is written to a real SQLite
/// Transparency Log, read back through the PUBLIC `query` surface — which
/// exercises the private `kind_to_string` map — and only then classified.
/// 16-2's kind arms already name kind 15; this keeps a Worker's stall rows
/// out of the call feed against future renumbering on either side.
#[test]
fn a_real_kind15_task_stalled_row_read_back_through_query_is_a_non_call() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let db = tmp.path().join("worker-stalled.sqlite");
    let conn = Connection::open(&db).expect("open SQLite");
    conn.execute_batch(SCHEMA_SQL).expect("schema init");
    conn.execute(
        "INSERT INTO transparency_log \
         (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0x2Au8; 16] as &[u8], // frame_id
            7_000i64,               // timestamp_ns
            4242i64,                // spirit_pid — the WORKER's pid
            0xCAFE_F00Di64,         // boot_nonce
            Option::<&[u8]>::None,  // capability_token — NULL, the scheduler writes none
            15i64,                  // kind — task.stalled
            "task.stalled",
            br#"{"task_id":"task-worker-1","stalled_ns":1000}"# as &[u8],
            0i64,                   // origin
        ],
    )
    .expect("insert task.stalled row");

    let entries = query(&db, AuditFilter::default()).expect("query succeeds");
    assert_eq!(entries.len(), 1, "exactly one seeded row");
    assert_eq!(
        entries[0].kind, "task.stalled",
        "kind 15 must render as task.stalled on the read side"
    );
    assert_eq!(
        classify_fr4_row(&entries[0]),
        Fr4RowDisposition::NonCallKernelEvent,
        "the Worker's stall row must stay out of the call feed"
    );
}
