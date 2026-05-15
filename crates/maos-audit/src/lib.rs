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
    /// FR4 schema-projection rejected an entry. Surface side stops emitting
    /// on the first violation (AC2: fail fast; no silent pass on partial coverage).
    #[error("FR4 schema violation at line {line}: missing field '{missing_field}'")]
    Fr4SchemaViolation {
        line: usize,
        missing_field: &'static str,
    },
}

/// FR4-projection error returned by [`project_to_fr4`]. Lifted into
/// [`AuditError::Fr4SchemaViolation`] by [`to_fr4_ndjson`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Fr4SchemaError {
    /// `capability_token` was `NULL` — mandatory under FR4's 100% mediation rule.
    #[error("missing capability_token")]
    MissingCapabilityToken,
    /// `kind` discriminator decoded to `unknown(N)` — projection refuses to
    /// emit a row whose call_type the read side does not understand.
    #[error("unknown call_type '{0}'")]
    UnknownCallType(String),
}

impl Fr4SchemaError {
    /// Stable, short string naming the missing field (used in diagnostics
    /// and in [`AuditError::Fr4SchemaViolation::missing_field`]).
    pub fn missing_field(&self) -> &'static str {
        match self {
            Fr4SchemaError::MissingCapabilityToken => "capability_token",
            Fr4SchemaError::UnknownCallType(_) => "call_type",
        }
    }
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
        params.push(Box::new(limit as i64));
        sql.push_str(" LIMIT ?");
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
///
/// This is the raw audit-entry surface preserved for Story 9.1 (subject-access /
/// posture-delta / sealed-export). For the FR4 mechanical-verification surface
/// see [`to_fr4_ndjson`].
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

// ─────────────────────────────────────────────────────────────────────────
// FR4 projection (Story 1b.5b, AC1)
// ─────────────────────────────────────────────────────────────────────────

/// FR4 NDJSON projection of a Transparency-Log row.
///
/// Per AC1, every entry surfaced by `maosctl audit query --spirit <name>` must
/// carry exactly these six keys and all five **mandatory** fields
/// (`capability_token`, `spirit_pid`, `boot_nonce`, `call_type`, `timestamp_ns`)
/// must be non-null. A missing or null mandatory field is a schema violation
/// that fails the command with exit code 2 — see [`to_fr4_ndjson`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Fr4Entry {
    /// 32-char hex of the 16-byte frame_id (always present — PRIMARY KEY in schema).
    pub call_id: String,
    /// 64-char hex of the 32-byte Ed25519 capability token.
    /// Mandatory — `None` upstream becomes [`Fr4SchemaError::MissingCapabilityToken`].
    pub capability_token: String,
    /// Spirit process ID at the time of the call. Mandatory.
    pub spirit_pid: u32,
    /// Boot nonce of the kernel that wrote this row. Mandatory.
    pub boot_nonce: u64,
    /// Dot-separated kind string (e.g. `"capability.invocation"`,
    /// `"inference.call"`). Mandatory; `unknown(N)` is rejected.
    pub call_type: String,
    /// Wall-clock timestamp in nanoseconds. Mandatory.
    pub timestamp_ns: u64,
}

/// Project a raw [`AuditEntry`] to the FR4 schema. Returns
/// [`Fr4SchemaError::MissingCapabilityToken`] when the source row has
/// `capability_token = NULL`, and [`Fr4SchemaError::UnknownCallType`] when
/// `kind` decoded to `unknown(N)`.
pub fn project_to_fr4(entry: &AuditEntry) -> Result<Fr4Entry, Fr4SchemaError> {
    let capability_token = entry
        .capability_token_hex
        .clone()
        .ok_or(Fr4SchemaError::MissingCapabilityToken)?;
    if entry.kind.starts_with("unknown(") {
        return Err(Fr4SchemaError::UnknownCallType(entry.kind.clone()));
    }
    Ok(Fr4Entry {
        call_id: entry.frame_id_hex.clone(),
        capability_token,
        spirit_pid: entry.spirit_pid,
        boot_nonce: entry.boot_nonce,
        call_type: entry.kind.clone(),
        timestamp_ns: entry.timestamp_ns,
    })
}

/// Write entries as FR4 NDJSON — one [`Fr4Entry`] per line.
///
/// Per AC1 + AC2, stops at the first projection failure and returns
/// [`AuditError::Fr4SchemaViolation`] naming the offending 1-indexed line and
/// the missing field. Output is buffered internally so no partial NDJSON lines
/// reach the writer on violation — the dispatcher must surface the error and
/// exit non-zero.
pub fn to_fr4_ndjson<W: Write>(
    entries: impl IntoIterator<Item = AuditEntry>,
    mut out: W,
) -> Result<(), AuditError> {
    let mut buf = Vec::new();
    for (idx, entry) in entries.into_iter().enumerate() {
        let projected = project_to_fr4(&entry).map_err(|e| AuditError::Fr4SchemaViolation {
            line: idx + 1,
            missing_field: e.missing_field(),
        })?;
        let line = serde_json::to_string(&projected)?;
        writeln!(buf, "{line}")?;
    }
    out.write_all(&buf)?;
    Ok(())
}

/// Write entries as human-readable tabular text. Never emits ANSI escapes
/// (no `colored` crate, no `\x1b` bytes). Used by `maosctl audit query
/// --format plain` and engaged automatically when the NFR-Ops-5 cascade
/// disables color (`--plain` / `NO_COLOR=1` / `TERM=dumb`).
///
/// Rows missing a `capability_token` are rendered as `<missing>` in the
/// token column rather than skipped — the operator sees the gap directly.
pub fn to_plain<W: Write>(
    entries: impl IntoIterator<Item = AuditEntry>,
    mut out: W,
) -> Result<(), AuditError> {
    writeln!(
        out,
        "{:<32}  {:<16}  {:<10}  {:<22}  {:<20}  {}",
        "call_id", "boot_nonce", "spirit_pid", "call_type", "timestamp_ns", "capability_token",
    )?;
    for entry in entries {
        let token = entry
            .capability_token_hex
            .as_deref()
            .unwrap_or("<missing>");
        writeln!(
            out,
            "{:<32}  {:016x}  {:<10}  {:<22}  {:<20}  {}",
            truncate(&entry.frame_id_hex, 32),
            entry.boot_nonce,
            entry.spirit_pid,
            truncate(&entry.kind, 22),
            entry.timestamp_ns,
            token,
        )?;
    }
    Ok(())
}

/// Write entries as FR4-validated human-readable tabular text. Same as
/// [`to_plain`] but validates the FR4 mandatory-field contract first and
/// aborts with [`AuditError::Fr4SchemaViolation`] on the first violation.
/// Used when `--spirit` is active and `--format plain` is selected so that
/// both formats enforce the same exit-code-2 contract per AC1.
pub fn to_fr4_plain<W: Write>(
    entries: impl IntoIterator<Item = AuditEntry>,
    mut out: W,
) -> Result<(), AuditError> {
    let projected: Vec<Fr4Entry> = entries
        .into_iter()
        .enumerate()
        .map(|(idx, entry)| {
            project_to_fr4(&entry).map_err(|e| AuditError::Fr4SchemaViolation {
                line: idx + 1,
                missing_field: e.missing_field(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    writeln!(
        out,
        "{:<32}  {:<16}  {:<10}  {:<22}  {:<20}",
        "call_id", "boot_nonce", "spirit_pid", "call_type", "timestamp_ns",
    )?;
    for entry in &projected {
        writeln!(
            out,
            "{:<32}  {:016x}  {:<10}  {:<22}  {:<20}",
            truncate(&entry.call_id, 32),
            entry.boot_nonce,
            entry.spirit_pid,
            truncate(&entry.call_type, 22),
            entry.timestamp_ns,
        )?;
    }
    Ok(())
}

/// Resolve the default Transparency Log SQLite path.
///
/// Shared by `maos-bin` (write side) and `maos-cli` (read side) so both
/// binaries always agree on the same location. Extracted here rather than
/// duplicated across crates to prevent silent path-drift data loss.
///
/// Precedence (highest → lowest):
///   1. `MAOS_AUDIT_DB` env var (explicit override; used by tests and ops).
///      Empty-string is rejected — callers should exit with a diagnostic.
///   2. `$XDG_DATA_HOME/maos/audit/transparency.sqlite`
///   3. `$HOME/.local/share/maos/audit/transparency.sqlite`
///   4. `/var/lib/maos/audit/transparency.sqlite` (last-resort fallback)
pub fn default_transparency_log_path() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("MAOS_AUDIT_DB") {
        if p.is_empty() {
            eprintln!("maos: MAOS_AUDIT_DB is set but empty — unset it or provide a path");
            std::process::exit(2);
        }
        return PathBuf::from(p);
    }
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("audit").join("transparency.sqlite")
}

fn truncate(s: &str, n: usize) -> &str {
    let mut end = n.min(s.len());
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
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

    // ── FR4 projection + writer tests (Story 1b.5b, AC1) ────────────────

    fn sample_entry() -> AuditEntry {
        AuditEntry {
            frame_id_hex: "aa".repeat(16),
            timestamp_ns: 1_700_000_000_000_000_000,
            spirit_pid: 7,
            boot_nonce: 0xCAFE_F00D_DEAD_BEEF,
            capability_token_hex: Some("bb".repeat(32)),
            kind: "inference.call".into(),
            intent: "claude-3-haiku".into(),
        }
    }

    #[test]
    fn project_to_fr4_keeps_five_mandatory_fields() {
        let projected = project_to_fr4(&sample_entry()).unwrap();
        assert_eq!(projected.call_id.len(), 32);
        assert_eq!(projected.capability_token.len(), 64);
        assert_eq!(projected.spirit_pid, 7);
        assert_eq!(projected.boot_nonce, 0xCAFE_F00D_DEAD_BEEF);
        assert_eq!(projected.call_type, "inference.call");
        assert_eq!(projected.timestamp_ns, 1_700_000_000_000_000_000);
    }

    #[test]
    fn project_to_fr4_rejects_null_capability_token() {
        let mut entry = sample_entry();
        entry.capability_token_hex = None;
        let err = project_to_fr4(&entry).unwrap_err();
        assert_eq!(err, Fr4SchemaError::MissingCapabilityToken);
        assert_eq!(err.missing_field(), "capability_token");
    }

    #[test]
    fn project_to_fr4_rejects_unknown_call_type() {
        let mut entry = sample_entry();
        entry.kind = "unknown(42)".into();
        let err = project_to_fr4(&entry).unwrap_err();
        assert!(matches!(err, Fr4SchemaError::UnknownCallType(_)));
        assert_eq!(err.missing_field(), "call_type");
    }

    #[test]
    fn to_fr4_ndjson_emits_exact_schema_keys() {
        let mut buf = Vec::new();
        to_fr4_ndjson(vec![sample_entry()], &mut buf).unwrap();
        let line = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let obj = parsed.as_object().expect("object");
        // Exactly the six keys, no extras (intent, payload_redacted excluded).
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort();
        assert_eq!(
            keys,
            vec!["boot_nonce", "call_id", "call_type", "capability_token", "spirit_pid", "timestamp_ns"]
        );
    }

    #[test]
    fn to_fr4_ndjson_stops_on_first_violation_with_line_number() {
        let mut good = sample_entry();
        good.frame_id_hex = "11".repeat(16);
        let mut bad = sample_entry();
        bad.frame_id_hex = "22".repeat(16);
        bad.capability_token_hex = None;
        let mut buf = Vec::new();
        let err = to_fr4_ndjson(vec![good, bad], &mut buf).unwrap_err();
        match err {
            AuditError::Fr4SchemaViolation { line, missing_field } => {
                assert_eq!(line, 2);
                assert_eq!(missing_field, "capability_token");
            }
            _ => panic!("expected Fr4SchemaViolation"),
        }
        // Buffer is flushed only on success, so no lines are emitted on violation.
        let written = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(lines.len(), 0, "no partial output on FR4 schema violation");
    }

    #[test]
    fn to_plain_emits_zero_ansi_bytes() {
        let mut buf = Vec::new();
        to_plain(vec![sample_entry()], &mut buf).unwrap();
        let esc_count = buf.iter().filter(|b| **b == 0x1b).count();
        assert_eq!(esc_count, 0, "to_plain emitted ANSI escape bytes");
        // Header + 1 data row.
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s.lines().count(), 2);
        assert!(s.contains("call_id"));
    }

    #[test]
    fn to_plain_renders_missing_capability_token_inline() {
        let mut entry = sample_entry();
        entry.capability_token_hex = None;
        let mut buf = Vec::new();
        to_plain(vec![entry], &mut buf).unwrap();
        let esc_count = buf.iter().filter(|b| **b == 0x1b).count();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("<missing>"));
        assert_eq!(esc_count, 0);
    }
}
