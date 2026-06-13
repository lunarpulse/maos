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

pub mod erasure;
pub mod log_composition;
pub mod replay;

pub mod sealed_export;

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
    /// A u64 value exceeded i64 range when converting for SQLite binding.
    #[error("value overflow: {field} ({value}) exceeds i64 range")]
    ValueOverflow { field: &'static str, value: u64 },
    /// Empty capability filter string provided.
    #[error("empty capability filter string")]
    EmptyCapabilityFilter,
    /// A `kind` filter string does not map to a known frame kind.
    #[error("unknown frame kind filter '{0}'")]
    UnknownKind(String),
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
    /// Per-frame redaction metadata. Populated ONLY by `query_with_redaction()`.
    /// Mirrors the `capability_token_hex` serde-skip pattern (F1 A-prime).
    #[serde(rename = "redaction", skip_serializing_if = "Option::is_none", default)]
    pub redaction: Option<RedactionMeta>,
}

/// Redaction metadata for a single frame (Story 9.2b, F1 A-prime).
///
/// Carries the redaction class + **bucketed** original payload byte length
/// (power-of-two bucket, NOT exact byte count).  No content hash — see
/// ADR-028 D3 (F5: confirmation-oracle risk on low-entropy fields).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RedactionMeta {
    /// Redaction class derived from frame metadata (e.g. "payload", "intent").
    pub class: String,
    /// Bucketed original payload byte length (power-of-two bucket).
    pub original_len_bucket: u64,
}

/// Filter for the read-side query — same shape as the kernel-side
/// `FrameFilter` but isolated in this crate.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub spirit_pid: Option<u32>,
    pub boot_nonce: Option<u64>,
    pub kind: Option<String>,
    pub since_ns: Option<u64>,
    pub until_ns: Option<u64>,
    pub limit: Option<usize>,
    /// FR41 — exact-match on capability_token BLOB (hex-encoded input).
    pub capability_token: Option<String>,
    /// FR41 — param-bound substring match on intent column.
    pub intent_contains: Option<String>,
}

/// Open the per-Host SQLite file read-only and return matching entries.
pub fn query(db_path: &Path, filter: AuditFilter) -> Result<Vec<AuditEntry>, AuditError> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(AuditError::Open)?;

    let mut sql = String::from(
        "SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce,
                capability_token, kind, intent
         FROM transparency_log",
    );
    let mut where_clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(pid) = filter.spirit_pid {
        where_clauses.push("spirit_pid = ?".to_string());
        let pid_i64 = i64::try_from(pid).map_err(|_| AuditError::ValueOverflow {
            field: "spirit_pid",
            value: pid.into(),
        })?;
        params.push(Box::new(pid_i64));
    }
    if let Some(boot) = filter.boot_nonce {
        where_clauses.push("boot_nonce = ?".to_string());
        let boot_i64 = i64::try_from(boot).map_err(|_| AuditError::ValueOverflow {
            field: "boot_nonce",
            value: boot,
        })?;
        params.push(Box::new(boot_i64));
    }
    if let Some(since) = filter.since_ns {
        where_clauses.push("timestamp_ns >= ?".to_string());
        let since_i64 = i64::try_from(since).map_err(|_| AuditError::ValueOverflow {
            field: "since_ns",
            value: since,
        })?;
        params.push(Box::new(since_i64));
    }
    if let Some(until) = filter.until_ns {
        where_clauses.push("timestamp_ns <= ?".to_string());
        let until_i64 = i64::try_from(until).map_err(|_| AuditError::ValueOverflow {
            field: "until_ns",
            value: until,
        })?;
        params.push(Box::new(until_i64));
    }
    if let Some(kind_str) = &filter.kind {
        match kind_from_string(kind_str) {
            Some(kind_int) => {
                where_clauses.push("kind = ?".to_string());
                params.push(Box::new(kind_int));
            }
            None => return Err(AuditError::UnknownKind(kind_str.clone())),
        }
    }
    // FR41 — capability_token: hex input → blob comparison (param-bound).
    if let Some(hex_str) = &filter.capability_token {
        if hex_str.is_empty() {
            return Err(AuditError::EmptyCapabilityFilter);
        }
        let blob = hex::decode(hex_str)
            .map_err(|e| AuditError::Read(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))?;
        where_clauses.push("capability_token = ?".to_string());
        params.push(Box::new(blob));
    }
    // FR41 — intent_contains: param-bound LIKE, never string-interpolated (SQLi-safe).
    if let Some(sub) = &filter.intent_contains {
        where_clauses.push("intent LIKE '%' || ? || '%'".to_string());
        params.push(Box::new(sub.clone()));
    }

    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY timestamp_ns ASC, frame_id ASC");
    if let Some(limit) = filter.limit {
        let limit_i64 = i64::try_from(limit).map_err(|_| AuditError::ValueOverflow {
            field: "limit",
            value: limit as u64,
        })?;
        params.push(Box::new(limit_i64));
        sql.push_str(" LIMIT ?");
    }

    let mut stmt = conn.prepare(&sql).map_err(AuditError::Read)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
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
                redaction: None,
            })
        })
        .map_err(AuditError::Read)?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(AuditError::Read)?);
    }
    Ok(entries)
}

/// Bucket a byte length to a privacy-safe size bucket.
///
/// Used by `query_with_redaction()` to produce length buckets for redacted
/// payloads (ADR-028 D3 — no exact byte length, no content hash).
///
/// To avoid a confirmation oracle on low-entropy fields (e.g. a boolean),
/// values below 8 bytes are rounded up to the 8-byte bucket so that many
/// distinct small lengths collide. Zero-length payloads bucket to 0.
pub fn bucket_len(len: usize) -> u64 {
    if len == 0 {
        return 0;
    }
    std::cmp::max((len as u64).next_power_of_two(), 8)
}

/// Query the Transparency Log with redaction metadata populated.
///
/// Same as [`query`] but additionally reads `payload_redacted`. For rows where
/// the column is non-empty, it derives a privacy-safe `RedactionMeta`; rows
/// with an empty `payload_redacted` keep `AuditEntry::redaction = None`. This
/// is the **ONLY** sanctioned populator of `AuditEntry::redaction` — see
/// ADR-028 D4 and the call-path oracle test
/// `redaction_field_is_none_for_all_non_replay_callers`.
///
/// Requires the same read-only SQLite connection as `query()`.
pub fn query_with_redaction(
    db_path: &Path,
    filter: AuditFilter,
) -> Result<Vec<AuditEntry>, AuditError> {
    // ADR-028 D6: replay determinism requires a quiesced / WAL-checkpointed DB.
    // If a SQLite WAL file is present, the DB may still have uncheckpointed
    // writes from an open writer, making replay non-deterministic.
    let wal_path = db_path.with_extension("sqlite-wal");
    if wal_path.exists() {
        return Err(AuditError::Open(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ffi::ErrorCode::DatabaseBusy,
                extended_code: 0,
            },
            Some(format!(
                "Transparency Log has an active WAL ({}). \
                 Quiesce/checkpoint the kernel before deterministic replay/export.",
                wal_path.display()
            )),
        )));
    }

    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(AuditError::Open)?;
    let mut sql = String::from(
        "SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce,
                capability_token, kind, intent, payload_redacted
         FROM transparency_log",
    );
    let mut where_clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(pid) = filter.spirit_pid {
        where_clauses.push("spirit_pid = ?".to_string());
        let pid_i64 = i64::try_from(pid).map_err(|_| AuditError::ValueOverflow {
            field: "spirit_pid",
            value: pid.into(),
        })?;
        params.push(Box::new(pid_i64));
    }
    if let Some(boot) = filter.boot_nonce {
        where_clauses.push("boot_nonce = ?".to_string());
        let boot_i64 = i64::try_from(boot).map_err(|_| AuditError::ValueOverflow {
            field: "boot_nonce",
            value: boot,
        })?;
        params.push(Box::new(boot_i64));
    }
    if let Some(since) = filter.since_ns {
        where_clauses.push("timestamp_ns >= ?".to_string());
        let since_i64 = i64::try_from(since).map_err(|_| AuditError::ValueOverflow {
            field: "since_ns",
            value: since,
        })?;
        params.push(Box::new(since_i64));
    }
    if let Some(until) = filter.until_ns {
        where_clauses.push("timestamp_ns <= ?".to_string());
        let until_i64 = i64::try_from(until).map_err(|_| AuditError::ValueOverflow {
            field: "until_ns",
            value: until,
        })?;
        params.push(Box::new(until_i64));
    }
    if let Some(kind_str) = &filter.kind {
        match kind_from_string(kind_str) {
            Some(kind_int) => {
                where_clauses.push("kind = ?".to_string());
                params.push(Box::new(kind_int));
            }
            None => return Err(AuditError::UnknownKind(kind_str.clone())),
        }
    }
    if let Some(hex_str) = &filter.capability_token {
        if hex_str.is_empty() {
            return Err(AuditError::EmptyCapabilityFilter);
        }
        let blob = hex::decode(hex_str)
            .map_err(|e| AuditError::Read(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))?;
        where_clauses.push("capability_token = ?".to_string());
        params.push(Box::new(blob));
    }
    if let Some(limit) = filter.limit {
        let limit_i64 = i64::try_from(limit).map_err(|_| AuditError::ValueOverflow {
            field: "limit",
            value: limit as u64,
        })?;
        params.push(Box::new(limit_i64));
        sql.push_str(" LIMIT ?");
    }
    if let Some(sub) = &filter.intent_contains {
        where_clauses.push("intent LIKE '%' || ? || '%'".to_string());
        params.push(Box::new(sub.clone()));
    }

    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY timestamp_ns ASC, frame_id ASC");
    let mut stmt = conn.prepare(&sql).map_err(AuditError::Read)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            let frame_id_blob: Vec<u8> = row.get(0)?;
            let cap_blob: Option<Vec<u8>> = row.get(4)?;
            let payload_blob: Vec<u8> = row.get(7)?;
            let kind_int: i64 = row.get(5)?;
            let kind_str = kind_to_string(kind_int);

            let redaction = if payload_blob.is_empty() {
                None
            } else {
                Some(RedactionMeta {
                    class: kind_str.clone(),
                    original_len_bucket: bucket_len(payload_blob.len()),
                })
            };
            Ok(AuditEntry {
                frame_id_hex: hex_encode(&frame_id_blob),
                timestamp_ns: row.get::<_, i64>(1)? as u64,
                spirit_pid: row.get::<_, i64>(2)? as u32,
                boot_nonce: row.get::<_, i64>(3)? as u64,
                capability_token_hex: cap_blob.as_ref().map(|b| hex_encode(b)),
                kind: kind_str,
                intent: row.get(6)?,
                redaction,
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

/// Truncate a string display to `max_len` characters.
/// Respects Unicode character boundaries — never splits a multi-byte code point.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        s[..end].to_string()
    }
}

/// Hex-encode a byte slice.
fn hex_encode(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Convert a kind integer to a human-readable dot-case string.
/// Stable format used by `maosctl audit query --format plain`.
fn kind_to_string(kind: i64) -> String {
    match kind {
        0 => "task.assign",
        1 => "task.complete",
        2 => "decision.dispatch",
        3 => "epistemic.halt",
        4 => "telemetry.event",
        5 => "consent.request",
        6 => "retract",
        7 => "capability.invocation",
        8 => "sandbox.block",
        9 => "inference.call",
        10 => "decision",
        11 => "distillate",
        17 => "spirit.revoked",
        19 => "spirit.admitted",
        22 => "consent.rupture",
        _ => "unknown",
    }
    .to_string()
}

/// Convert a kind string to its integer discriminator.
/// Accepts both dot-case (`"task.assign"`) and PascalCase (`"TaskAssign"`) for backward compat.
fn kind_from_string(s: &str) -> Option<i64> {
    match s {
        "task.assign" | "TaskAssign" => Some(0),
        "task.complete" | "TaskComplete" => Some(1),
        "decision.dispatch" | "DecisionDispatch" => Some(2),
        "epistemic.halt" | "EpistemicHalt" => Some(3),
        "telemetry.event" | "TelemetryEvent" => Some(4),
        "consent.request" | "ConsentRequest" => Some(5),
        "retract" | "Retract" => Some(6),
        "capability.invocation" | "CapabilityInvocation" => Some(7),
        "sandbox.block" | "SandboxBlock" => Some(8),
        "inference.call" | "InferenceCall" => Some(9),
        "decision" | "Decision" => Some(10),
        "distillate" | "Distillate" => Some(11),
        "spirit.revoked" | "SpiritRevoked" => Some(17),
        "spirit.admitted" | "SpiritAdmitted" => Some(19),
        "consent.rupture" | "ConsentRupture" => Some(22),
        _ => None,
    }
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
        let token = entry.capability_token_hex.as_deref().unwrap_or("<missing>");
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
///   1. `MAOS_HOME` env var (Story 8.14a FORK 5 — init'd home).
///      If set, path = `$MAOS_HOME/audit/transparency.sqlite`.
///   2. `MAOS_AUDIT_DB` env var (explicit override; used by tests and ops).
///      Empty-string is rejected — callers should exit with a diagnostic.
///   3. `$XDG_DATA_HOME/maos/audit/transparency.sqlite`
///   4. `$HOME/.local/share/maos/audit/transparency.sqlite`
///   5. `/var/lib/maos/audit/transparency.sqlite` (last-resort fallback)
pub fn default_transparency_log_path() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(home) = std::env::var("MAOS_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home)
                .join("audit")
                .join("transparency.sqlite");
        }
    }
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
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home
        .join("maos")
        .join("audit")
        .join("transparency.sqlite")
}

/// Resolve the default Lifecycle Journal NDJSON path.
///
/// Shared by `maos-bin` (write side — lifecycle verbs in the
/// `MAOS_ONE_SHOT={start, stop, unload}` one-shot path, Story 1b.5c)
/// and any reader (e.g. operator inspection, Story 5.x supervisor).
/// Extracted here rather than duplicated across crates to prevent
/// silent path-drift data loss — the same discipline established by
/// [`default_transparency_log_path`] (Story 1b.5b D2).
///
/// Precedence (highest → lowest):
///   1. `MAOS_HOME` env var (Story 8.14a FORK 5 — init'd home).
///      If set, path = `$MAOS_HOME/journal/lifecycle.ndjson`.
///   2. `MAOS_JOURNAL_PATH` env var (explicit override; used by tests
///      and ops). Empty-string is rejected — callers exit 2 with a
///      diagnostic (same shape as [`default_transparency_log_path`]).
///   3. `$XDG_DATA_HOME/maos/journal/lifecycle.ndjson`
///   4. `$HOME/.local/share/maos/journal/lifecycle.ndjson`
///   5. `/var/lib/maos/journal/lifecycle.ndjson` (last-resort fallback)
///
/// File suffix is `.ndjson` to match the Journal's NDJSON-on-disk
/// storage choice (Story 1b.1 / `journal/mod.rs` §"Storage choice").
pub fn default_journal_path() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(home) = std::env::var("MAOS_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join("journal").join("lifecycle.ndjson");
        }
    }
    if let Ok(p) = std::env::var("MAOS_JOURNAL_PATH") {
        if p.is_empty() {
            eprintln!("maos: MAOS_JOURNAL_PATH is set but empty — unset it or provide a path");
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
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home
        .join("maos")
        .join("journal")
        .join("lifecycle.ndjson")
}

/// Resolve the default Memory Root directory.
///
/// Shared by `maos-bin` (write side) and any reader (e.g. operator
/// inspection). Precedence (highest → lowest):
///   1. `MAOS_MEMORY_ROOT` env var
///   2. `$XDG_DATA_HOME/maos/memory`
///   3. `$HOME/.local/share/maos/memory`
///   4. `/var/lib/maos/memory` (last-resort fallback)
pub fn default_memory_root() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("MAOS_MEMORY_ROOT") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
        // Empty env var treated as unset — fall through to next precedence.
        eprintln!("maos: MAOS_MEMORY_ROOT is set but empty — falling through to default path");
    }
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("memory")
}

/// Resolve the default Distillate Corpus root directory.
///
/// Forward-shaped helper for v0.5+ when the corpus may live in operator-supplied
/// data directories outside the repo. Precedence (highest → lowest):
///   1. `MAOS_DISTILLATE_CORPUS_ROOT` env var
///   2. `$XDG_DATA_HOME/maos/distillate-corpus`
///   3. `$HOME/.local/share/maos/distillate-corpus`
///   4. `/var/lib/maos/distillate-corpus` (last-resort fallback)
///
/// # Note
///
/// The kernel does NOT consume this function itself; the harness in
/// `maos-eval/tests/` reads from a relative fixture path
/// (`fixtures/distillate-corpus-v0/`) consistent with the existing
/// `halt-corpus-v0` test pattern. This function exists for v0.5+.
pub fn default_distillate_corpus_root() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("MAOS_DISTILLATE_CORPUS_ROOT") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
        eprintln!(
            "maos: MAOS_DISTILLATE_CORPUS_ROOT is set but empty — falling through to default path"
        );
    }
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("distillate-corpus")
}

/// Resolve the default Isolation Corpus root directory (Story 4.5).
///
/// Mirrors [`default_distillate_corpus_root`]. Precedence:
///   1. `MAOS_ISOLATION_CORPUS_ROOT` env var
///   2. `$XDG_DATA_HOME/maos/isolation-corpus`
///   3. `$HOME/.local/share/maos/isolation-corpus`
///   4. `/var/lib/maos/isolation-corpus`
pub fn default_isolation_corpus_root() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("MAOS_ISOLATION_CORPUS_ROOT") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
        eprintln!(
            "maos: MAOS_ISOLATION_CORPUS_ROOT is set but empty — falling through to default path"
        );
    }
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("isolation-corpus")
}

/// Resolve the default Spirit Archive root directory (Story 5.2).
///
/// Precedence (highest → lowest):
///   1. `MAOS_ARCHIVE_DIR` env var
///   2. `$XDG_DATA_HOME/maos/spirit-archives`
///   3. `$HOME/.local/share/maos/spirit-archives`
///   4. `/var/lib/maos/spirit-archives`
pub fn default_archive_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("MAOS_ARCHIVE_DIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
        eprintln!("maos: MAOS_ARCHIVE_DIR is set but empty — falling through to default path");
    }
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("spirit-archives")
}
/// Resolve the default erasure-proof retention directory (Story 9.2).
///
/// Precedence (highest → lowest):
///   1. `MAOS_ERASURE_PROOFS_DIR` env var
///   2. `$XDG_DATA_HOME/maos/erasure-proofs`
///   3. `$HOME/.local/share/maos/erasure-proofs`
///   4. `/var/lib/maos/erasure-proofs`
pub fn default_erasure_proofs_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("MAOS_ERASURE_PROOFS_DIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
        // P19: an empty MAOS_ERASURE_PROOFS_DIR silently falls through to the
        // default path — a read-only library path-resolver must not perform
        // side-effecting I/O on a recoverable misconfiguration.
    }
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("erasure-proofs")
}
/// Resolve a Spirit name to one or more `(boot_nonce, spirit_pid)` pairs by
/// scanning the Transparency Log for SpiritAdmitted (FrameKind 19) frames
/// whose non-redacted `intent` column carries the Spirit name.
///
/// Per Decision E: keyed on `(boot_nonce, spirit_pid)` to discriminate pid
/// reuse across boots. Defaults to the LATEST boot (max `boot_nonce`).
/// Set `all_boots = true` to union all incarnations.
///
/// Returns `Err` if no matching frames exist (unknown spirit name).
pub fn resolve_spirit_name(
    db_path: &Path,
    name: &str,
    all_boots: bool,
) -> Result<Vec<(u64, u32)>, String> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("failed to open TL: {e}"))?;

    // Decision E: scan FrameKind 19 (SpiritAdmitted) TL frames.
    // The Spirit name is stored in the non-redacted `intent` column.
    let sql = "SELECT boot_nonce, spirit_pid, intent
               FROM transparency_log
               WHERE kind = 19
               ORDER BY timestamp_ns ASC";
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("prepare failed: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let boot: i64 = row.get(0)?;
            let pid: i64 = row.get(1)?;
            let intent: String = row.get(2)?;
            Ok((boot as u64, pid as u32, intent))
        })
        .map_err(|e| format!("query failed: {e}"))?;

    let mut matches: Vec<(u64, u32)> = Vec::new();
    for row in rows {
        let (boot, pid, intent) = row.map_err(|e| format!("row error: {e}"))?;
        if intent == name {
            matches.push((boot, pid));
        }
    }

    if matches.is_empty() {
        // Backward-compat fallback for the v0.1-β evaluator path:
        // `MAOS_ONE_SHOT=hello-spirit` predates kernel-side FrameKind 19
        // emission, so if no admission frames exist but the TL contains
        // rows for the legacy hardcoded hello-spirit PID, resolve from
        // those rows rather than failing. This keeps Story 1b.5b's FR4
        // smoke path working while still preferring Decision E authoritative
        // admissions when present.
        if name == "hello-spirit" {
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT boot_nonce, spirit_pid
                     FROM transparency_log
                     WHERE spirit_pid = 0
                     ORDER BY boot_nonce ASC",
                )
                .map_err(|e| format!("prepare fallback failed: {e}"))?;
            let rows = stmt
                .query_map([], |row| {
                    let boot: i64 = row.get(0)?;
                    let pid: i64 = row.get(1)?;
                    Ok((boot as u64, pid as u32))
                })
                .map_err(|e| format!("fallback query failed: {e}"))?;
            for row in rows {
                matches.push(row.map_err(|e| format!("fallback row error: {e}"))?);
            }
        }

        if matches.is_empty() {
            return Err(format!(
                "unknown spirit '{name}' — no SpiritAdmitted (kind=19) frame found in Transparency Log"
            ));
        }
    }

    if all_boots {
        // Deduplicate by (boot_nonce, spirit_pid)
        matches.sort();
        matches.dedup();
        Ok(matches)
    } else {
        // Default: latest boot (max boot_nonce)
        let max_boot = matches.iter().map(|(b, _)| *b).max().unwrap();
        let latest: Vec<(u64, u32)> = matches
            .into_iter()
            .filter(|(b, _)| *b == max_boot)
            .collect();
        Ok(latest)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// FR42 — Subject-access query (principal_index reader)
// ─────────────────────────────────────────────────────────────────────────

/// One row from the `principal_index` table. Independently defined to keep
/// the dep direction clean (no maos-kernel-core import).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PrincipalIndexEntry {
    pub principal_id: String,
    pub writer_spirit_pid: u32,
    pub schema: String,
    pub key: String,
    pub timestamp_ns: u64,
}

/// Query the `principal_index` table for all rows matching a given principal.
/// Opens the same SQLite file as the TL (read-only).
pub fn subject_access_query(
    db_path: &Path,
    principal_id: &str,
) -> Result<Vec<PrincipalIndexEntry>, AuditError> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(AuditError::Open)?;

    let sql = "SELECT principal_id, writer_spirit_pid, schema, key, timestamp_ns
               FROM principal_index
               WHERE principal_id = ?
               ORDER BY timestamp_ns ASC";
    let mut stmt = conn.prepare(sql).map_err(AuditError::Read)?;
    let rows = stmt
        .query_map(rusqlite::params![principal_id], |row| {
            Ok(PrincipalIndexEntry {
                principal_id: row.get(0)?,
                writer_spirit_pid: row.get::<_, i64>(1)? as u32,
                schema: row.get(2)?,
                key: row.get(3)?,
                timestamp_ns: row.get::<_, i64>(4)? as u64,
            })
        })
        .map_err(AuditError::Read)?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(AuditError::Read)?);
    }
    Ok(entries)
}

/// Provenance type for subject-access enrichment (Decision D: Direct/Distilled).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "provenance_type")]
pub enum Provenance {
    Direct {
        frame_ref: String,
    },
    Distilled {
        effective_source_log_ref: Vec<String>,
        distillation_depth: u32,
    },
}

/// Enriched subject-access entry with provenance and spirit-name resolution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SubjectAccessEntry {
    pub principal_id: String,
    pub writer_spirit_pid: u32,
    pub writer_spirit_name: Option<String>,
    pub boot_nonce: Option<u64>,
    pub schema: String,
    pub key: String,
    pub timestamp_ns: u64,
    pub provenance: Provenance,
}

/// Enrich a `PrincipalIndexEntry` with provenance by scanning the TL for
/// Distillate frames whose `spirit_pid` matches the writer. Each Distillate
/// frame carries `effective_source_log_ref` in `payload_redacted`.
///
/// Falls back to `Provenance::Direct` when no Distillate frame is found.
pub fn enrich_subject_access(
    db_path: &Path,
    entries: Vec<PrincipalIndexEntry>,
) -> Result<Vec<SubjectAccessEntry>, AuditError> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(AuditError::Open)?;

    // Build a (spirit_pid, boot_nonce) → (name, admit_timestamp_ns) map from
    // SpiritAdmitted frames (kind = 19). Keyed by (pid, boot) to handle pid
    // reuse across boots; admission timestamp lets us pick the correct
    // incarnation for each principal_index entry.
    let mut spirit_names: std::collections::HashMap<(u32, u64), (String, u64)> =
        std::collections::HashMap::new();
    {
        let sql = "SELECT spirit_pid, boot_nonce, intent, timestamp_ns
                   FROM transparency_log
                   WHERE kind = 19
                   ORDER BY timestamp_ns ASC";
        let mut stmt = conn.prepare(sql).map_err(AuditError::Read)?;
        let rows = stmt
            .query_map([], |row| {
                let pid: i64 = row.get(0)?;
                let boot: i64 = row.get(1)?;
                let intent: String = row.get(2)?;
                let admit_ts: i64 = row.get(3)?;
                Ok((pid as u32, boot as u64, intent, admit_ts as u64))
            })
            .map_err(AuditError::Read)?;
        for row in rows {
            let (pid, boot, intent, admit_ts) = row.map_err(AuditError::Read)?;
            spirit_names
                .entry((pid, boot))
                .or_insert((intent, admit_ts));
        }
    }
    // Scan for Distillate frames to build provenance.
    // Latest distillate for a given (pid, boot) wins; ordered by timestamp ASC.
    let mut distillate_map: std::collections::HashMap<(u32, u64), (Vec<String>, u32)> =
        std::collections::HashMap::new();
    {
        let sql = "SELECT spirit_pid, boot_nonce, frame_id, payload_redacted
                   FROM transparency_log
                   WHERE intent LIKE 'distillate%'
                   ORDER BY timestamp_ns ASC";
        let mut stmt = conn.prepare(sql).map_err(AuditError::Read)?;
        let rows = stmt
            .query_map([], |row| {
                let pid: i64 = row.get(0)?;
                let boot: i64 = row.get(1)?;
                let frame_id: Vec<u8> = row.get(2)?;
                let payload: Option<Vec<u8>> = row.get(3)?;
                Ok((pid as u32, boot as u64, frame_id, payload))
            })
            .map_err(AuditError::Read)?;
        for row in rows {
            let (pid, boot, frame_id, payload) = row.map_err(AuditError::Read)?;
            if let Some(ref payload_bytes) = payload {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(payload_bytes) {
                    let refs = val
                        .get("effective_source_log_ref")
                        .and_then(|v| v.as_str())
                        .map(|s| s.split(':').map(|r| r.to_string()).collect())
                        .unwrap_or_else(|| vec![hex_encode(&frame_id)]);
                    let depth = val
                        .get("distillation_depth")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as u32;
                    // Latest distillate for this (pid, boot) wins — ordered by timestamp ASC.
                    distillate_map.insert((pid, boot), (refs, depth));
                }
            }
        }
    }

    let mut enriched = Vec::with_capacity(entries.len());
    for entry in entries {
        // Per-entry provenance: find the Spirit incarnation that was active
        // when this entry was written. Admissions are keyed by (pid, boot);
        // the latest admission whose timestamp is <= entry.timestamp_ns wins.
        // This correctly attributes entries under pid reuse across boots.
        let (name, boot_nonce) = {
            let mut admissions: Vec<(u64, String)> = spirit_names
                .iter()
                .filter(|((pid, _), _)| *pid == entry.writer_spirit_pid)
                .filter(|(_, (_, admit_ts))| *admit_ts <= entry.timestamp_ns)
                .map(|((_, boot), (name, _))| (*boot, name.clone()))
                .collect();
            admissions.sort_by_key(|(boot, _)| *boot);
            admissions
                .last()
                .map(|(boot, name)| (Some(name.clone()), Some(*boot)))
                .unwrap_or((None, None))
        };

        let provenance = match boot_nonce {
            Some(boot) => match distillate_map.get(&(entry.writer_spirit_pid, boot)) {
                Some((refs, depth)) => Provenance::Distilled {
                    effective_source_log_ref: refs.clone(),
                    distillation_depth: *depth,
                },
                None => Provenance::Direct {
                    frame_ref: format!("{}:{}", entry.schema, entry.key),
                },
            },
            None => Provenance::Direct {
                frame_ref: format!("{}:{}", entry.schema, entry.key),
            },
        };

        enriched.push(SubjectAccessEntry {
            principal_id: entry.principal_id,
            writer_spirit_pid: entry.writer_spirit_pid,
            writer_spirit_name: name,
            boot_nonce,
            schema: entry.schema,
            key: entry.key,
            timestamp_ns: entry.timestamp_ns,
            provenance,
        });
    }
    Ok(enriched)
}

/// Pure-function form of the precedence cascade — env values are passed in
/// explicitly. Used by the inline tests on [`default_journal_path`] to drive
/// every branch without mutating the process environment (forbidden under
/// `#![forbid(unsafe_code)]` since Rust's env-mutation API became `unsafe`).
#[cfg(test)]
fn resolve_journal_path_from_env_internal(
    maos_journal_path: Option<&str>,
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Some(p) = maos_journal_path {
        if p.is_empty() {
            panic!("empty MAOS_JOURNAL_PATH");
        }
        return PathBuf::from(p);
    }
    let data_home = xdg_data_home
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home
        .join("maos")
        .join("journal")
        .join("lifecycle.ndjson")
}

/// Pure-function form of the memory-root precedence cascade for testing.
#[cfg(test)]
fn resolve_memory_root_from_env_internal(
    maos_memory_root: Option<&str>,
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Some(p) = maos_memory_root {
        if p.is_empty() {
            panic!("empty MAOS_MEMORY_ROOT");
        }
        return PathBuf::from(p);
    }
    let data_home = xdg_data_home
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("memory")
}

/// Pure-function form of the distillate-corpus-root precedence cascade for testing.
#[cfg(test)]
fn resolve_distillate_corpus_root_from_env_internal(
    maos_corpus_root: Option<&str>,
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Some(p) = maos_corpus_root {
        if p.is_empty() {
            panic!("empty MAOS_DISTILLATE_CORPUS_ROOT");
        }
        return PathBuf::from(p);
    }
    let data_home = xdg_data_home
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("distillate-corpus")
}

/// Pure-function form of the isolation-corpus-root precedence cascade for testing.
#[cfg(test)]
fn resolve_isolation_corpus_root_from_env_internal(
    maos_corpus_root: Option<&str>,
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> std::path::PathBuf {
    use std::path::PathBuf;
    if let Some(p) = maos_corpus_root {
        if p.is_empty() {
            panic!("empty MAOS_ISOLATION_CORPUS_ROOT");
        }
        return PathBuf::from(p);
    }
    let data_home = xdg_data_home
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    data_home.join("maos").join("isolation-corpus")
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
            );",
        )
        .unwrap();
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
            );",
        )
        .unwrap();
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
            redaction: None,
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
            redaction: None,
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
            vec![
                "boot_nonce",
                "call_id",
                "call_type",
                "capability_token",
                "spirit_pid",
                "timestamp_ns"
            ]
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
            AuditError::Fr4SchemaViolation {
                line,
                missing_field,
            } => {
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

    // ── default_journal_path tests (Story 1b.5c, Task 1) ────────────────
    //
    // The resolver reads `MAOS_JOURNAL_PATH`, `XDG_DATA_HOME`, `HOME` from
    // the process environment. We can't mutate process env safely here —
    // the crate `#![forbid(unsafe_code)]` rules out the (now `unsafe`)
    // `std::env::set_var` / `remove_var` API. Instead we exercise the
    // resolver's precedence by spawning a subprocess via `cargo test`'s
    // injected binary harness — but that requires a binary crate, which
    // `maos-audit` is not.
    //
    // Pragmatic path: split the resolution logic into an env-injected
    // pure function and drive it from the three #[test]s. The exported
    // `default_journal_path` is a thin wrapper that reads process env
    // and delegates. This is the same discipline 1b.5b would have used
    // had `default_transparency_log_path` been tested inline.

    #[test]
    fn default_journal_path_respects_env_override() {
        let p = super::resolve_journal_path_from_env_internal(
            Some("/tmp/maos-test-journal.ndjson"),
            None,
            None,
        );
        assert_eq!(p, std::path::PathBuf::from("/tmp/maos-test-journal.ndjson"));
    }

    #[test]
    fn default_journal_path_falls_through_to_xdg() {
        let p = super::resolve_journal_path_from_env_internal(None, Some("/tmp/xdgtest"), None);
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/xdgtest/maos/journal/lifecycle.ndjson")
        );
    }

    #[test]
    fn default_journal_path_falls_through_to_home_when_xdg_unset() {
        let p = super::resolve_journal_path_from_env_internal(None, None, Some("/tmp/hometest"));
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/hometest/.local/share/maos/journal/lifecycle.ndjson")
        );
    }

    #[test]
    fn default_journal_path_last_resort_var_lib() {
        // Both XDG and HOME unset → /var/lib fallback (the production
        // path Story 5.x supervisors land on if XDG_DATA_HOME and HOME
        // are both absent in the systemd unit's environment).
        let p = super::resolve_journal_path_from_env_internal(None, None, None);
        assert_eq!(
            p,
            std::path::PathBuf::from("/var/lib/maos/journal/lifecycle.ndjson")
        );
    }

    // ── default_memory_root tests (Story 4.3) ────────────────────────

    #[test]
    fn default_memory_root_respects_env_override() {
        let p =
            super::resolve_memory_root_from_env_internal(Some("/tmp/maos-test-memory"), None, None);
        assert_eq!(p, std::path::PathBuf::from("/tmp/maos-test-memory"));
    }

    #[test]
    fn default_memory_root_falls_through_to_xdg() {
        let p = super::resolve_memory_root_from_env_internal(None, Some("/tmp/xdgtest"), None);
        assert_eq!(p, std::path::PathBuf::from("/tmp/xdgtest/maos/memory"));
    }

    #[test]
    fn default_memory_root_falls_through_to_home_when_xdg_unset() {
        let p = super::resolve_memory_root_from_env_internal(None, None, Some("/tmp/hometest"));
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/hometest/.local/share/maos/memory")
        );
    }

    #[test]
    fn default_memory_root_last_resort_var_lib() {
        let p = super::resolve_memory_root_from_env_internal(None, None, None);
        assert_eq!(p, std::path::PathBuf::from("/var/lib/maos/memory"));
    }

    // ── default_distillate_corpus_root tests (Story 4.4) ────────────────────

    #[test]
    fn default_distillate_corpus_root_respects_env_override() {
        let p = super::resolve_distillate_corpus_root_from_env_internal(
            Some("/tmp/maos-test-distillate-corpus"),
            None,
            None,
        );
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/maos-test-distillate-corpus")
        );
    }

    #[test]
    fn default_distillate_corpus_root_falls_through_to_xdg() {
        let p = super::resolve_distillate_corpus_root_from_env_internal(
            None,
            Some("/tmp/xdgtest"),
            None,
        );
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/xdgtest/maos/distillate-corpus")
        );
    }

    #[test]
    fn default_distillate_corpus_root_falls_through_to_home_when_xdg_unset() {
        let p = super::resolve_distillate_corpus_root_from_env_internal(
            None,
            None,
            Some("/tmp/hometest"),
        );
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/hometest/.local/share/maos/distillate-corpus")
        );
    }

    #[test]
    fn default_distillate_corpus_root_last_resort_var_lib() {
        let p = super::resolve_distillate_corpus_root_from_env_internal(None, None, None);
        assert_eq!(
            p,
            std::path::PathBuf::from("/var/lib/maos/distillate-corpus")
        );
    }

    // ── default_isolation_corpus_root tests (Story 4.5) ────────────────────────

    #[test]
    fn default_isolation_corpus_root_respects_env_override() {
        let p = super::resolve_isolation_corpus_root_from_env_internal(
            Some("/tmp/isolation-corpus"),
            None,
            None,
        );
        assert_eq!(p, std::path::PathBuf::from("/tmp/isolation-corpus"));
    }

    #[test]
    fn default_isolation_corpus_root_falls_through_to_xdg() {
        let p = super::resolve_isolation_corpus_root_from_env_internal(
            None,
            Some("/tmp/xdgtest"),
            None,
        );
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/xdgtest/maos/isolation-corpus")
        );
    }

    #[test]
    fn default_isolation_corpus_root_falls_through_to_home_when_xdg_unset() {
        let p = super::resolve_isolation_corpus_root_from_env_internal(
            None,
            None,
            Some("/tmp/hometest"),
        );
        assert_eq!(
            p,
            std::path::PathBuf::from("/tmp/hometest/.local/share/maos/isolation-corpus")
        );
    }

    #[test]
    fn default_isolation_corpus_root_last_resort_var_lib() {
        let p = super::resolve_isolation_corpus_root_from_env_internal(None, None, None);
        assert_eq!(
            p,
            std::path::PathBuf::from("/var/lib/maos/isolation-corpus")
        );
    }

    // ── FR41 SQLi-inert test ─────────────────────────────────────────

    /// Helper: create a test DB with one row whose intent is "delegate".
    fn sqli_test_db() -> (tempfile::TempDir, std::path::PathBuf) {
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
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0xAAu8; 16] as &[u8],
                1000i64,
                7i64,
                1i64,
                &[0xBBu8; 32] as &[u8],
                7i64,
                "delegate",
                b"redacted_payload" as &[u8],
                0i64,
            ],
        ).unwrap();
        drop(conn);
        (tmpdir, db_path)
    }

    #[test]
    fn intent_contains_sqli_is_inert() {
        let (_tmpdir, db_path) = sqli_test_db();
        // A SQLi attempt must NOT match anything — it's param-bound.
        let sqli = "%' OR 1=1 --";
        let mut filter = AuditFilter::default();
        filter.intent_contains = Some(sqli.to_string());
        let entries = query(&db_path, filter).unwrap();
        // The DB has one row with intent="delegate". The SQLi string is
        // treated as a literal substring — it does NOT appear in "delegate",
        // so the result must be empty.
        assert!(
            entries.is_empty(),
            "SQLi attempt must match literally, not inject SQL; got {} results",
            entries.len()
        );
    }

    #[test]
    fn intent_contains_matches_substring() {
        let (_tmpdir, db_path) = sqli_test_db();
        let mut filter = AuditFilter::default();
        filter.intent_contains = Some("deleg".to_string());
        let entries = query(&db_path, filter).unwrap();
        assert_eq!(entries.len(), 1, "substring 'deleg' must match 'delegate'");
    }

    #[test]
    fn capability_token_filter_works() {
        let (_tmpdir, db_path) = sqli_test_db();
        let mut filter = AuditFilter::default();
        filter.capability_token = Some("bb".repeat(32));
        let entries = query(&db_path, filter).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "matching capability_token must find the row"
        );

        // Non-matching token
        let mut filter2 = AuditFilter::default();
        filter2.capability_token = Some("cc".repeat(32));
        let entries2 = query(&db_path, filter2).unwrap();
        assert!(
            entries2.is_empty(),
            "non-matching capability_token must return empty"
        );
    }

    #[test]
    fn boot_nonce_filter_works() {
        let (_tmpdir, db_path) = sqli_test_db();
        let mut filter = AuditFilter::default();
        filter.boot_nonce = Some(1);
        let entries = query(&db_path, filter).unwrap();
        assert_eq!(entries.len(), 1);

        let mut filter2 = AuditFilter::default();
        filter2.boot_nonce = Some(999);
        let entries2 = query(&db_path, filter2).unwrap();
        assert!(entries2.is_empty());
    }

    // ── FR42 subject_access_query tests ────────────────────────────────

    #[test]
    fn subject_access_query_returns_matching_rows() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let db_path = tmpdir.path().join("test.sqlite");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS principal_index (
                principal_id TEXT NOT NULL,
                writer_spirit_pid INTEGER NOT NULL,
                schema TEXT NOT NULL,
                key TEXT NOT NULL,
                timestamp_ns INTEGER NOT NULL,
                PRIMARY KEY (principal_id, writer_spirit_pid, schema, key)
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["user:alice", 7i64, "memory", "session-1", 1000i64],
        ).unwrap();
        conn.execute(
            "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["user:bob", 8i64, "memory", "session-2", 2000i64],
        ).unwrap();
        drop(conn);

        let entries = subject_access_query(&db_path, "user:alice").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].principal_id, "user:alice");
        assert_eq!(entries[0].writer_spirit_pid, 7);
        assert_eq!(entries[0].schema, "memory");
    }

    #[test]
    fn resolve_spirit_name_finds_admitted_spirit() {
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
            );",
        )
        .unwrap();
        let payload = serde_json::to_vec(&serde_json::json!({"spirit_id": "researcher"})).unwrap();
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0x01u8; 16] as &[u8],
                1000i64,
                42i64,
                5i64,
                rusqlite::types::Null,
                19i64,  // SpiritAdmitted
                "researcher",
                &payload as &[u8],
                0i64,
            ],
        ).unwrap();
        drop(conn);

        let result = resolve_spirit_name(&db_path, "researcher", false).unwrap();
        assert_eq!(result, vec![(5, 42)]);
    }

    #[test]
    fn resolve_spirit_name_rejects_unknown() {
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
            );",
        )
        .unwrap();
        drop(conn);

        let err = resolve_spirit_name(&db_path, "nonexistent", false).unwrap_err();
        assert!(err.contains("unknown spirit 'nonexistent'"));
    }

    // ── FR42 enrich_subject_access provenance tests ────────────────────

    /// Helper: create a test SQLite file with both `transparency_log` and
    /// `principal_index` tables. Returns `(tmpdir, db_path)` — the tmpdir
    /// must stay alive for the file to exist.
    fn create_test_db_with_principal_index() -> (tempfile::TempDir, std::path::PathBuf) {
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
            );
            CREATE TABLE IF NOT EXISTS principal_index (
                principal_id TEXT NOT NULL,
                writer_spirit_pid INTEGER NOT NULL,
                schema TEXT NOT NULL,
                key TEXT NOT NULL,
                timestamp_ns INTEGER NOT NULL,
                PRIMARY KEY (principal_id, writer_spirit_pid, schema, key)
            );",
        )
        .unwrap();
        drop(conn);
        (tmpdir, db_path)
    }

    #[test]
    fn enrich_subject_access_direct_provenance() {
        let (_tmpdir, db_path) = create_test_db_with_principal_index();
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        // principal_index row: alice written by spirit pid=42
        conn.execute(
            "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["alice@example.org", 42i64, "memory", "session-1", 1000i64],
        ).unwrap();

        // TL: lifecycle.admit for spirit pid=42 so enrich can resolve the name
        let admit_payload = serde_json::to_vec(&serde_json::json!({
            "spirit_id": "researcher"
        }))
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0x01u8; 16] as &[u8],
                500i64,  // before the principal entry
                42i64,
                10i64,
                rusqlite::types::Null,
                19i64,  // SpiritAdmitted
                "researcher",
                &admit_payload as &[u8],
                0i64,
            ],
        ).unwrap();

        // NO Distillate frames for pid 42 — provenance must be Direct
        drop(conn);

        let entries = subject_access_query(&db_path, "alice@example.org").unwrap();
        assert_eq!(entries.len(), 1);
        let enriched = enrich_subject_access(&db_path, entries).unwrap();
        assert_eq!(enriched.len(), 1);

        // Must be Direct provenance since no Distillate frame exists
        assert!(
            matches!(&enriched[0].provenance, Provenance::Direct { frame_ref } if frame_ref == "memory:session-1"),
            "expected Direct provenance with frame_ref 'memory:session-1', got {:?}",
            enriched[0].provenance,
        );
        assert_eq!(
            enriched[0].writer_spirit_name.as_deref(),
            Some("researcher")
        );
        assert_eq!(enriched[0].boot_nonce, Some(10));
    }

    #[test]
    fn enrich_subject_access_distilled_provenance() {
        let (_tmpdir, db_path) = create_test_db_with_principal_index();
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        // principal_index row: bob written by spirit pid=99
        conn.execute(
            "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["bob@example.org", 99i64, "preference", "theme", 2000i64],
        ).unwrap();

        // TL: lifecycle.admit for spirit pid=99
        let admit_payload = serde_json::to_vec(&serde_json::json!({
            "spirit_id": "butler"
        }))
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0x01u8; 16] as &[u8],
                500i64,
                99i64,
                20i64,  // boot_nonce
                rusqlite::types::Null,
                19i64,  // SpiritAdmitted
                "butler",
                &admit_payload as &[u8],
                0i64,
            ],
        ).unwrap();

        // TL: Distillate frame for pid=99, boot_nonce=20
        let distillate_payload = serde_json::to_vec(&serde_json::json!({
            "kind": "distillate",
            "effective_source_log_ref": "ab:cd:ef:12:34",
            "distillation_depth": 2
        }))
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0x02u8; 16] as &[u8],
                1000i64,
                99i64,
                20i64,  // same boot_nonce as lifecycle.admit
                rusqlite::types::Null,
                11i64,  // Distillate
                "distillate.commit",
                &distillate_payload as &[u8],
                0i64,
            ],
        ).unwrap();

        drop(conn);

        let entries = subject_access_query(&db_path, "bob@example.org").unwrap();
        assert_eq!(entries.len(), 1);
        let enriched = enrich_subject_access(&db_path, entries).unwrap();
        assert_eq!(enriched.len(), 1);

        // Must be Distilled provenance with correct depth and refs
        match &enriched[0].provenance {
            Provenance::Distilled {
                effective_source_log_ref,
                distillation_depth,
            } => {
                assert_eq!(*distillation_depth, 2);
                assert_eq!(
                    effective_source_log_ref,
                    &vec![
                        "ab".to_string(),
                        "cd".to_string(),
                        "ef".to_string(),
                        "12".to_string(),
                        "34".to_string(),
                    ]
                );
            }
            other => panic!("expected Distilled provenance, got {:?}", other),
        }
        assert_eq!(enriched[0].writer_spirit_name.as_deref(), Some("butler"));
        assert_eq!(enriched[0].boot_nonce, Some(20));
    }

    #[test]
    fn pid_reuse_misattribution_test() {
        let (_tmpdir, db_path) = create_test_db_with_principal_index();
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        // Spirit "researcher" admitted at pid=100, boot_nonce=1000
        let researcher_payload = serde_json::to_vec(&serde_json::json!({
            "spirit_id": "researcher"
        }))
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0x01u8; 16] as &[u8],
                1000i64,
                100i64,
                1000i64,  // boot_nonce for researcher
                rusqlite::types::Null,
                19i64,  // SpiritAdmitted
                "researcher",
                &researcher_payload as &[u8],
                0i64,
            ],
        ).unwrap();

        // Spirit "butler" reused pid=100, boot_nonce=2000
        let butler_payload = serde_json::to_vec(&serde_json::json!({
            "spirit_id": "butler"
        }))
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0x02u8; 16] as &[u8],
                2000i64,
                100i64,  // SAME PID
                2000i64,  // different boot_nonce
                rusqlite::types::Null,
                19i64,
                "butler",
                &butler_payload as &[u8],
                0i64,
            ],
        ).unwrap();

        // principal_index: alice's entries written by researcher (pid=100)
        conn.execute(
            "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["alice@example.org", 100i64, "memory", "research-note-1", 1200i64],
        ).unwrap();

        // principal_index: bob's entries written by butler (pid=100, same pid)
        conn.execute(
            "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["bob@example.org", 100i64, "preference", "theme", 2200i64],
        ).unwrap();

        drop(conn);

        // Query alice's data — should get ONLY researcher's entry
        let entries = subject_access_query(&db_path, "alice@example.org").unwrap();
        assert_eq!(entries.len(), 1, "alice should have exactly one entry");
        assert_eq!(entries[0].key, "research-note-1");

        // Enrich: each entry is attributed to the incarnation active at its
        // write timestamp. Alice wrote at ts=1200, between researcher (ts=1000)
        // and butler (ts=2000), so she is attributed to researcher boot=1000.
        let enriched = enrich_subject_access(&db_path, entries).unwrap();
        assert_eq!(enriched.len(), 1);
        assert_eq!(
            enriched[0].writer_spirit_name.as_deref(),
            Some("researcher")
        );
        assert_eq!(enriched[0].boot_nonce, Some(1000));

        // Bob wrote at ts=2200, after butler's admission, so he is attributed
        // to butler boot=2000 — the previous pid-only logic would have wrongly
        // attributed him to researcher.
        let bob_entries = subject_access_query(&db_path, "bob@example.org").unwrap();
        assert_eq!(bob_entries.len(), 1);
        assert_eq!(bob_entries[0].key, "theme");
        let bob_enriched = enrich_subject_access(&db_path, bob_entries).unwrap();
        assert_eq!(bob_enriched.len(), 1);
        assert_eq!(
            bob_enriched[0].writer_spirit_name.as_deref(),
            Some("butler")
        );
        assert_eq!(bob_enriched[0].boot_nonce, Some(2000));
    }
}
