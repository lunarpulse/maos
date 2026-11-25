#![forbid(unsafe_code)]

//! Story 22-5 — the read-only session view behind `GET /v1/spirits/{id}/session` and the
//! age and spend columns of `GET /v1/agents`.
//!
//! Every function opens its own `READ_ONLY | NOFOLLOW` connection and runs bounded
//! `SELECT`s; nothing here can write a row. Rows are **header only** (D-22-5-08 A): the
//! payload column is read solely to build a typed `summary` and is never returned. An
//! Approval Decision Log row is reduced to actor, capability, the decision and the halt
//! resolution *kind*; the operator's `provided_context` text never leaves this module.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::{hex_encode, kind_to_string, truncate, AuditError};

/// Largest page a caller may ask for; the door bounds its response by this.
pub const MAX_SESSION_ROWS: usize = 500;

/// Where a follow poll resumes. `after_rowid: None` asks for the newest rows (the first
/// read); `Some` asks for the rows past the cursors, oldest first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionCursor {
    pub after_rowid: Option<u64>,
    pub after_decision: u64,
}

/// One printed session row. `source` is `tl` (Transparency Log; `rowid` is its rowid) or
/// `approval` (Approval Decision Log; `rowid` is its `decision_id`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SessionRow {
    pub source: &'static str,
    pub rowid: u64,
    pub timestamp_ns: u64,
    pub kind: String,
    pub intent: String,
    /// First 8 hex chars of the frame id (`-` for an approval row).
    pub frame_id: String,
    pub from: String,
    pub to: String,
    pub origin: i64,
    pub summary: String,
}

/// What `GET /v1/agents` reads per Spirit from the journal (current boot only).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgentVitals {
    /// The Spirit's first journaled row (its admission row), if any.
    pub first_row_ns: Option<u64>,
    pub tokens_in: u64,
    pub tokens_out: u64,
}

fn open(db: &Path) -> Result<Connection, AuditError> {
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(AuditError::Open)?;
    conn.busy_timeout(std::time::Duration::from_secs(2))
        .map_err(AuditError::Open)?;
    Ok(conn)
}

fn to_i64(field: &'static str, value: u64) -> Result<i64, AuditError> {
    i64::try_from(value).map_err(|_| AuditError::ValueOverflow { field, value })
}

/// Printable, tab-free text: bytes outside `[A-Za-z0-9._:/@-]` become `?`, cut at `max`.
fn clean(text: &str, max: usize) -> String {
    truncate(text, max)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '/' | '@' | '-') {
                c
            } else {
                '?'
            }
        })
        .collect()
}

/// The first row and the token spend of each pid under `boot`, in `pids` order.
pub fn agent_vitals(db: &Path, boot: u64, pids: &[u32]) -> Result<Vec<AgentVitals>, AuditError> {
    let conn = open(db)?;
    let boot = to_i64("boot_nonce", boot)?;
    let mut first = conn
        .prepare(
            "SELECT MIN(timestamp_ns) FROM transparency_log WHERE boot_nonce = ?1 AND spirit_pid = ?2",
        )
        .map_err(AuditError::Read)?;
    let mut spend = conn
        .prepare(
            "SELECT COALESCE(SUM(json_extract(CAST(payload_redacted AS TEXT), '$.dimensions.tokens_in')), 0),
                    COALESCE(SUM(json_extract(CAST(payload_redacted AS TEXT), '$.dimensions.tokens_out')), 0)
             FROM transparency_log
             WHERE boot_nonce = ?1 AND spirit_pid = ?2 AND kind = 29
               AND json_valid(CAST(payload_redacted AS TEXT))",
        )
        .map_err(AuditError::Read)?;
    pids.iter()
        .map(|pid| {
            let first_row_ns = first
                .query_row(rusqlite::params![boot, i64::from(*pid)], |row| {
                    row.get::<_, Option<i64>>(0)
                })
                .map_err(AuditError::Read)?
                .map(|ns| ns as u64);
            let (tokens_in, tokens_out) = spend
                .query_row(rusqlite::params![boot, i64::from(*pid)], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(AuditError::Read)?;
            Ok(AgentVitals {
                first_row_ns,
                tokens_in: tokens_in.max(0) as u64,
                tokens_out: tokens_out.max(0) as u64,
            })
        })
        .collect()
}

/// The typed one-line summary of a row, built from parsed fields only (never raw payload).
fn summarize(
    kind: i64,
    payload: &[u8],
    classify_halt: &dyn Fn(&str) -> (&'static str, Option<String>),
) -> String {
    let text = String::from_utf8_lossy(payload);
    let json = serde_json::from_str::<serde_json::Value>(&text).ok();
    match kind {
        3 => {
            let (record, halt_id) = classify_halt(&text);
            let tag = json
                .as_ref()
                .and_then(|value| value.get("tag"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            format!(
                "record={record} halt_id={} tag={}",
                clean(halt_id.as_deref().unwrap_or("-"), 40),
                clean(tag, 64)
            )
        }
        29 => {
            let dimension = |name: &str| {
                json.as_ref()
                    .and_then(|value| value.get("dimensions")?.get(name)?.as_i64())
                    .unwrap_or(0)
            };
            format!(
                "tokens_in={} tokens_out={}",
                dimension("tokens_in"),
                dimension("tokens_out")
            )
        }
        22 => "consent_rupture".to_string(),
        _ => String::new(),
    }
}

/// `halt=<id>: <kind>[: …]` → `<kind>` when it is one of the three resolution kinds.
fn resolution_kind(reasoning: &str) -> Option<&'static str> {
    let (_, after) = reasoning.strip_prefix("halt=")?.split_once(": ")?;
    let head = after.split(':').next()?.trim();
    ["provided_context", "accepted_halt", "authorized_override"]
        .into_iter()
        .find(|kind| *kind == head)
}

/// The Spirit's session for boot `boot`: Transparency Log rows it emitted or received
/// (`transparency_log_recipients`) plus Approval Decision Log rows targeting `spirit_id` since
/// the Spirit's own first row. Oldest first. `limit` is clamped to
/// `1..=`[`MAX_SESSION_ROWS`]. Without `cursor.after_rowid` the newest `limit` rows are
/// returned; with it, the first `limit` rows past the cursors.
pub fn session_rows(
    db: &Path,
    boot: u64,
    pid: u32,
    spirit_id: &str,
    cursor: SessionCursor,
    limit: usize,
    classify_halt: &dyn Fn(&str) -> (&'static str, Option<String>),
) -> Result<Vec<SessionRow>, AuditError> {
    let limit = limit.clamp(1, MAX_SESSION_ROWS);
    let conn = open(db)?;
    let newest = cursor.after_rowid.is_none();
    let order = if newest { "DESC" } else { "ASC" };
    let boot_i = to_i64("boot_nonce", boot)?;
    let after = to_i64("after_rowid", cursor.after_rowid.unwrap_or(0))?;
    let limit_i = limit as i64;

    let sql = format!(
        "SELECT rid, frame_id, timestamp_ns, kind, intent, from_spirit_id, to_spirit_id, origin, payload_redacted FROM (
            SELECT rowid AS rid, frame_id, timestamp_ns, kind, intent, from_spirit_id, to_spirit_id, origin, payload_redacted
              FROM transparency_log WHERE boot_nonce = ?1 AND spirit_pid = ?3 AND rowid > ?2
            UNION
            SELECT t.rowid, t.frame_id, t.timestamp_ns, t.kind, t.intent, t.from_spirit_id, t.to_spirit_id, t.origin, t.payload_redacted
              FROM transparency_log_recipients r JOIN transparency_log t ON t.frame_id = r.frame_id
             WHERE r.recipient_pid = ?3 AND t.boot_nonce = ?1 AND t.rowid > ?2
         ) ORDER BY rid {order} LIMIT ?4"
    );
    let mut stmt = conn.prepare(&sql).map_err(AuditError::Read)?;
    let mapped = stmt
        .query_map(
            rusqlite::params![boot_i, after, i64::from(pid), limit_i],
            |row| {
                let frame: Vec<u8> = row.get(1)?;
                let kind: i64 = row.get(3)?;
                let payload: Vec<u8> = row.get(8)?;
                Ok(SessionRow {
                    source: "tl",
                    rowid: row.get::<_, i64>(0)? as u64,
                    timestamp_ns: row.get::<_, i64>(2)? as u64,
                    kind: kind_to_string(kind),
                    intent: clean(&row.get::<_, String>(4)?, 64),
                    frame_id: hex_encode(&frame).chars().take(8).collect(),
                    from: clean(&row.get::<_, String>(5)?, 64),
                    to: clean(&row.get::<_, String>(6)?, 64),
                    origin: row.get(7)?,
                    summary: summarize(kind, &payload, classify_halt),
                })
            },
        )
        .map_err(AuditError::Read)?;
    let mut rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(AuditError::Read)?;

    let floor: Option<i64> = conn
        .query_row(
            "SELECT MIN(timestamp_ns) FROM transparency_log WHERE boot_nonce = ?1 AND spirit_pid = ?2",
            rusqlite::params![boot_i, i64::from(pid)],
            |row| row.get(0),
        )
        .map_err(AuditError::Read)?;
    if let Some(floor) = floor {
        let sql = format!(
            "SELECT decision_id, timestamp_ns, actor, capability, decision, reasoning
               FROM approval_decision_log
              WHERE target = ?1 AND decision_id > ?2 AND timestamp_ns >= ?3
              ORDER BY decision_id {order} LIMIT ?4"
        );
        let mut stmt = conn.prepare(&sql).map_err(AuditError::Read)?;
        let mapped = stmt
            .query_map(
                rusqlite::params![
                    spirit_id,
                    to_i64("after_decision", cursor.after_decision)?,
                    floor,
                    limit_i
                ],
                |row| {
                    let granted = row.get::<_, i64>(4)? != 0;
                    let reasoning: Option<String> = row.get(5)?;
                    let resolution = reasoning.as_deref().and_then(resolution_kind);
                    Ok(SessionRow {
                        source: "approval",
                        rowid: row.get::<_, i64>(0)? as u64,
                        timestamp_ns: row.get::<_, i64>(1)? as u64,
                        kind: "approval".to_string(),
                        intent: clean(&row.get::<_, String>(3)?, 64),
                        frame_id: "-".to_string(),
                        from: clean(&row.get::<_, String>(2)?, 64),
                        to: clean(spirit_id, 64),
                        origin: 0,
                        summary: match resolution {
                            Some(kind) => format!(
                                "decision={} resolution={kind}",
                                if granted { "granted" } else { "denied" }
                            ),
                            None => {
                                format!("decision={}", if granted { "granted" } else { "denied" })
                            }
                        },
                    })
                },
            )
            .map_err(AuditError::Read)?;
        for row in mapped {
            rows.push(row.map_err(AuditError::Read)?);
        }
    }

    rows.sort_by(|a, b| {
        (a.timestamp_ns, a.source, a.rowid).cmp(&(b.timestamp_ns, b.source, b.rowid))
    });
    if newest {
        let excess = rows.len().saturating_sub(limit);
        rows.drain(..excess);
    } else {
        rows.truncate(limit);
    }
    Ok(rows)
}
