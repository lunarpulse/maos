#![forbid(unsafe_code)]

//! `maos-audit` — read-side SQLite query adapter for Transparency Log
//! + Approval Decision Log.
//!
//! This crate is read-only by design — it opens the SQLite file produced
//! by `maos-kernel-core::iac::transparency_log` with a read-only
//! connection (`SQLITE_OPEN_READ_ONLY` flag) and exposes query + NDJSON
//! export. The Story 1a.4 decoupling rule (`maos-cli` MUST NOT depend on
//! `maos-kernel-core`) is preserved by routing the CLI through this
//! separate crate; the kernel-core's write surface stays isolated.
//!
//! Story 9.1 extends this crate with subject-access, posture-delta, and
//! sealed-export functions.

use std::io::Write;
use std::path::Path;

use rusqlite::OpenFlags;

/// Typed audit-read error.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("sqlite open failed: {0}")]
    Open(rusqlite::Error),
    #[error("sqlite read failed: {0}")]
    Read(rusqlite::Error),
    #[error("ndjson encode failed: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// One audit entry from the Transparency Log. Mirrors the kernel-side
/// `TransparencyLogEntry` shape but is independently defined to keep
/// the dep direction clean (maos-audit depends on maos-domain only,
/// NOT on maos-kernel-core).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    /// 32-char hex of the 16-byte frame_id.
    #[serde(rename = "frame_id")]
    pub frame_id_hex: String,
    /// Monotonic wall-time nanoseconds.
    pub timestamp_ns: u64,
    /// Spirit process ID.
    pub spirit_pid: u32,
    /// Boot nonce of the kernel that wrote this entry.
    pub boot_nonce: u64,
    /// 64-char hex of the 32-byte Ed25519 capability token, if present.
    #[serde(rename = "capability_token", skip_serializing_if = "Option::is_none")]
    pub capability_token_hex: Option<String>,
    /// Frame kind as a dot-separated string (e.g. "task.assign").
    pub kind: String,
    /// Intent string from the frame.
    pub intent: String,
}

/// Filter for the read-side query — same shape as the kernel-side
/// `FrameFilter` but isolated in this crate.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub spirit_pid: Option<u32>,
    pub kind: Option<String>,
    pub since_ns: Option<u64>,
    pub until_ns: Option<u64>,
    pub limit: Option<usize>,
}

/// Open the per-Host SQLite file read-only and return matching entries.
pub fn query(
    db_path: &Path,
    filter: AuditFilter,
) -> Result<Vec<AuditEntry>, AuditError> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).map_err(AuditError::Open)?;

    let mut sql = String::from(
        "SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce,
                capability_token, kind, intent
         FROM transparency_log",
    );
    let mut where_clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(pid) = filter.spirit_pid {
        where_clauses.push("spirit_pid = ?".to_string());
        params.push(Box::new(pid as i64));
    }
    if let Some(since) = filter.since_ns {
        where_clauses.push("timestamp_ns >= ?".to_string());
        params.push(Box::new(since as i64));
    }
    if let Some(until) = filter.until_ns {
        where_clauses.push("timestamp_ns <= ?".to_string());
        params.push(Box::new(until as i64));
    }
    if let Some(kind_str) = &filter.kind {
        if let Some(kind_int) = kind_from_string(kind_str) {
            where_clauses.push("kind = ?".to_string());
            params.push(Box::new(kind_int));
        }
    }

    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY timestamp_ns ASC, frame_id ASC");
    if let Some(limit) = filter.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }

    let mut stmt = conn.prepare(&sql).map_err(AuditError::Read)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            let frame_id_blob: Vec<u8> = row.get(0)?;
            let cap_blob: Option<Vec<u8>> = row.get(4)?;
            Ok(AuditEntry {
                frame_id_hex: hex_encode(&frame_id_blob),
                timestamp_ns: row.get::<_, i64>(1)? as u64,
                spirit_pid: row.get::<_, i64>(2)? as u32,
                boot_nonce: row.get::<_, i64>(3)? as u64,
                capability_token_hex: cap_blob.as_ref().map(|b| hex_encode(b)),
                kind: kind_to_string(row.get::<_, i64>(5)?),
                intent: row.get(6)?,
            })
        })
        .map_err(AuditError::Read)?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(AuditError::Read)?);
    }
    Ok(entries)
}

/// Write entries to an NDJSON stream. One JSON object per line.
pub fn to_ndjson<W: Write>(
    entries: impl IntoIterator<Item = AuditEntry>,
    mut out: W,
) -> Result<(), AuditError> {
    for entry in entries {
        let line = serde_json::to_string(&entry)?;
        writeln!(out, "{line}")?;
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn kind_to_string(disc: i64) -> String {
    match disc {
        0 => "task.assign".into(),
        1 => "task.complete".into(),
        2 => "decision.dispatch".into(),
        3 => "epistemic.halt".into(),
        4 => "telemetry.event".into(),
        5 => "consent.request".into(),
        6 => "retract".into(),
        7 => "capability.invocation".into(),
        8 => "sandbox.block".into(),
        9 => "inference.call".into(),
        _ => format!("unknown({disc})"),
    }
}

fn kind_from_string(s: &str) -> Option<i64> {
    match s {
        "task.assign" => Some(0),
        "task.complete" => Some(1),
        "decision.dispatch" => Some(2),
        "epistemic.halt" => Some(3),
        "telemetry.event" => Some(4),
        "consent.request" => Some(5),
        "retract" => Some(6),
        "capability.invocation" => Some(7),
        "sandbox.block" => Some(8),
        "inference.call" => Some(9),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_empty_db_returns_empty() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let db_path = tmpdir.path().join("test.sqlite");

        // Create the schema using a write connection
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );"
        ).unwrap();
        drop(conn);

        let entries = query(&db_path, AuditFilter::default()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn query_returns_seeded_entries() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let db_path = tmpdir.path().join("test.sqlite");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );"
        ).unwrap();
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0xAAu8; 16] as &[u8],
                1000i64,
                7i64,
                0xDEADBEEFi64,
                &[0xBBu8; 32] as &[u8],
                7i64, // CapabilityInvocation
                "delegate",
                b"redacted_payload" as &[u8],
                0i64,
            ],
        ).unwrap();
        drop(conn);

        let entries = query(&db_path, AuditFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].spirit_pid, 7);
        assert_eq!(entries[0].boot_nonce, 0xDEADBEEF as u64);
        assert_eq!(entries[0].kind, "capability.invocation");
        assert!(entries[0].capability_token_hex.is_some());
        assert_eq!(entries[0].intent, "delegate");
    }

    #[test]
    fn to_ndjson_produces_valid_json() {
        let entries = vec![AuditEntry {
            frame_id_hex: "aa".repeat(16),
            timestamp_ns: 1000,
            spirit_pid: 7,
            boot_nonce: 0xDEAD_BEEF,
            capability_token_hex: Some("bb".repeat(32)),
            kind: "capability.invocation".into(),
            intent: "delegate".into(),
        }];
        let mut buf = Vec::new();
        to_ndjson(entries, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert_eq!(parsed["spirit_pid"], 7);
        assert_eq!(parsed["intent"], "delegate");
    }
}
