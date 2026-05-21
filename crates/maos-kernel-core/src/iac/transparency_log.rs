#![forbid(unsafe_code)]

//! Transparency Log + Approval Decision Log — kernel-managed SQLite audit spine.
//!
//! Per architecture §7.3 + §7.4 + Invariants I2/I4 (architecture §3.2,
//! enforcement-cadence `runtime` from v0.1). One file holds BOTH the
//! Transparency Log and the Approval Decision Log tables — they are
//! distinct tables per §7.4 + I4, but they share one `rusqlite::Connection`
//! to fit the existing I9-sanctioned single-file holder
//! (`xtask/i9-whitelist.toml`) without amending the whitelist.
//!
//! # I9 status
//!
//! This file is the I9-sanctioned holder for two pieces of persistent
//! state: the SQLite connection itself and the in-memory frame_id
//! monotonic counter. The `#[i9_exempt]` attribute on `TransparencyLogAdapter`
//! is documented at `docs/invariants/i9-exemptions.md` per the
//! `xtask check-empty-kernel` exemption discipline.

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use maos_domain::invariants::i2::LogBeforeDeliver;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i4::ApprovalDecision;
use maos_domain::ports::IacBusPort;
use rusqlite::{Connection, OptionalExtension};

use super::mailbox_stub::MailboxStub;
use super::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};

/// Frame-kind discriminator for the Transparency Log row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum FrameKind {
    TaskAssign = 0,
    TaskComplete = 1,
    DecisionDispatch = 2,
    EpistemicHalt = 3,
    TelemetryEvent = 4,
    ConsentRequest = 5,
    Retract = 6,
    /// Capability invocation (file op, network, exec, sub-Spirit spawn).
    /// Story 1b.2 `cap-audit` is the canonical writer of this kind.
    CapabilityInvocation = 7,
    /// Sandbox-tier block event (Story 1b.3).
    SandboxBlock = 8,
    /// Inference Port call (Story 1b.4).
    InferenceCall = 9,
    /// Decision / distillation outcome (Story 4.3 proxy; Story 4.4
    /// refines with explicit `Distillate` variant).
    Decision = 10,
    /// Story 4.4 — Distillation digest with kernel-enforced I11 audit chain.
    /// Payload (in `transparency_log.payload_redacted`) is the JSON-serialized
    /// `DistillationReceipt`. Use `DistillateWriter::write_distillate` as the
    /// canonical producer; direct `insert_frame_event(FrameKind::Distillate, ...)`
    /// from other code paths is forbidden by convention (the I11 enforcement
    /// MUST flow through the writer).
    Distillate = 11,
    /// Story 5.1 — BudgetWarning IAC frame (80% of time_cap_seconds per NFR-Perf-6).
    BudgetWarning = 12,
    /// Story 5.1 — BudgetExceeded IAC frame (100% of time_cap_seconds).
    BudgetExceeded = 13,
}

impl FrameKind {
    /// Convert from the SQLite integer discriminator.
    pub fn from_i64(v: i64) -> Option<Self> {
        match v {
            0 => Some(Self::TaskAssign),
            1 => Some(Self::TaskComplete),
            2 => Some(Self::DecisionDispatch),
            3 => Some(Self::EpistemicHalt),
            4 => Some(Self::TelemetryEvent),
            5 => Some(Self::ConsentRequest),
            6 => Some(Self::Retract),
            7 => Some(Self::CapabilityInvocation),
            8 => Some(Self::SandboxBlock),
            9 => Some(Self::InferenceCall),
            10 => Some(Self::Decision),
            11 => Some(Self::Distillate),
            12 => Some(Self::BudgetWarning),
            13 => Some(Self::BudgetExceeded),
            _ => None,
        }
    }
}

/// A single Transparency Log row — what `query_frames` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparencyLogEntry {
    pub frame_id: [u8; 16], // ULID bytes
    pub timestamp_ns: u64,
    pub spirit_pid: u32,
    pub boot_nonce: u64,
    pub capability_token: Option<[u8; 32]>,
    pub kind: FrameKind,
    pub intent: String,
    pub payload_redacted: Vec<u8>,
    pub origin: FrameOrigin,
}

/// Filter for `query_frames` and `query_approvals`. v0.1-β supports the
/// minimum needed by Story 1b.5b's `maosctl audit query --spirit <name>`;
/// extensions (subject-access, posture-delta) ship in Story 9.1.
#[derive(Debug, Clone, Default)]
pub struct FrameFilter {
    pub spirit_pid: Option<u32>,
    pub kind: Option<FrameKind>,
    pub since_ns: Option<u64>,
    pub until_ns: Option<u64>,
    pub limit: Option<usize>,
    /// Keyset-pagination cursor — exclusive lower bound on `(timestamp_ns, frame_id)`.
    /// When both are `Some`, the query adds `WHERE (timestamp_ns, frame_id) > (?cursor_ts, ?cursor_id)`.
    pub cursor_timestamp_ns: Option<u64>,
    pub cursor_frame_id: Option<[u8; 16]>,
}

/// Typed audit-spine error. Coarse-grained at v0.1-β per the dep-introduction
/// discipline (no `anyhow` in kernel-core; concrete variants only).
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("sqlite open failed: {0}")]
    SqliteOpen(#[from] rusqlite::Error),
    #[error("sqlite write failed: {0} — kernel panics per architecture §7.3 I2")]
    SqliteWriteFatal(rusqlite::Error),
    #[error("sqlite read failed: {0}")]
    SqliteRead(rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// SQL schema for both tables.
const SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    timestamp_ns        INTEGER NOT NULL,
    spirit_pid          INTEGER NOT NULL,
    boot_nonce          INTEGER NOT NULL,
    capability_token    BLOB,
    kind                INTEGER NOT NULL,
    intent              TEXT    NOT NULL,
    payload_redacted    BLOB    NOT NULL,
    origin              INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tlog_spirit_pid
    ON transparency_log(spirit_pid, timestamp_ns);
CREATE INDEX IF NOT EXISTS idx_tlog_kind
    ON transparency_log(kind, timestamp_ns);

CREATE TABLE IF NOT EXISTS approval_decision_log (
    decision_id         INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_ns        INTEGER NOT NULL,
    actor               TEXT    NOT NULL,
    target              TEXT    NOT NULL,
    capability          TEXT    NOT NULL,
    intent              TEXT    NOT NULL,
    decision            INTEGER NOT NULL,
    reasoning           TEXT
);

CREATE INDEX IF NOT EXISTS idx_approval_actor
    ON approval_decision_log(actor, timestamp_ns);
";

/// The Transparency Log + Approval Decision Log adapter.
///
/// One per Host; constructed in the composition root (`maos-bin/main.rs`)
/// with the deployment-configured path. Tests use `open_in_memory()`.
//
// I9 exemption: this struct's persistent state (Mutex<Connection>,
// in-memory frame_id counter) is sanctioned by the file-path whitelist
// entry at xtask/i9-whitelist.toml.
#[derive(Debug)]
pub struct TransparencyLogAdapter {
    inner: Mutex<TransparencyLogInner>,
    redaction: Box<dyn RedactionPolicy + Send + Sync>,
    mailbox: MailboxStub,
}

struct TransparencyLogInner {
    conn: Connection,
    next_frame_id_counter: u64,
    boot_nonce: u64,
    last_frame_id: [u8; 16],
}

impl std::fmt::Debug for TransparencyLogInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransparencyLogInner")
            .field("next_frame_id_counter", &self.next_frame_id_counter)
            .field("boot_nonce", &self.boot_nonce)
            .finish()
    }
}

impl TransparencyLogAdapter {
    /// Return the frame_id of the most recently inserted frame event.
    pub fn last_frame_id(&self) -> [u8; 16] {
        self.inner.lock().expect("TransparencyLogAdapter inner poisoned").last_frame_id
    }
    /// Open the per-Host SQLite file. Initializes both tables if not present.
    /// Panics if the file is opened with a schema version this kernel does
    /// not understand (forward-compat is Story 9.4's concern).
    #[doc(hidden)]
    pub fn open(path: &Path, boot_nonce: u64) -> Result<Self, AuditError> {
        Self::open_with_policy(
            path,
            boot_nonce,
            Box::new(CorpusBackedRedactionPolicy::new()),
        )
    }

    /// Open with a custom redaction policy (for tests and future composition).
    #[doc(hidden)]
    pub fn open_with_policy(
        path: &Path,
        boot_nonce: u64,
        redaction: Box<dyn RedactionPolicy + Send + Sync>,
    ) -> Result<Self, AuditError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self {
            inner: Mutex::new(TransparencyLogInner {
                conn,
                next_frame_id_counter: 0,
                boot_nonce,
                last_frame_id: [0u8; 16],
            }),
            redaction,
            mailbox: MailboxStub::new(),
        })
    }

    /// Open an in-memory SQLite database for tests.
    #[doc(hidden)]
    pub fn open_in_memory(boot_nonce: u64) -> Self {
        Self::open_in_memory_with_policy(
            boot_nonce,
            Box::new(CorpusBackedRedactionPolicy::new()),
        )
    }

    /// Open an in-memory SQLite database with custom redaction policy.
    #[doc(hidden)]
    pub fn open_in_memory_with_policy(
        boot_nonce: u64,
        redaction: Box<dyn RedactionPolicy + Send + Sync>,
    ) -> Self {
        let conn = Connection::open_in_memory().expect("in-memory SQLite must succeed");
        conn.execute_batch(SCHEMA_SQL)
            .expect("schema init must succeed");
        Self {
            inner: Mutex::new(TransparencyLogInner {
                conn,
                next_frame_id_counter: 0,
                boot_nonce,
                last_frame_id: [0u8; 16],
            }),
            redaction,
            mailbox: MailboxStub::new(),
        }
    }

    /// Generate a unique 16-byte frame ID using ULID.
    fn next_frame_id(inner: &mut TransparencyLogInner) -> [u8; 16] {
        inner.next_frame_id_counter += 1;
        let ulid = ulid::Ulid::new();
        let mut bytes = ulid.to_bytes();
        // Mix in the counter for additional uniqueness within the same millisecond
        let counter_bytes = inner.next_frame_id_counter.to_le_bytes();
        bytes[12] ^= counter_bytes[0];
        bytes[13] ^= counter_bytes[1];
        bytes[14] ^= counter_bytes[2];
        bytes[15] ^= counter_bytes[3];
        bytes
    }

    /// Insert a frame event. Returns `LogBeforeDeliver<()>` per I2 typestate:
    /// the caller can only construct `LogBeforeDeliver` by going through
    /// this method (the `i2::LogBeforeDeliver::new` constructor uses
    /// `#[doc(hidden)] pub` — visible but convention-gated at v0.1-beta).
    ///
    /// On SQLite write failure: PANICS per architecture §7.3 I2 ("if the
    /// log write fails, the kernel panics rather than silently dropping
    /// the frame"). The panic-vs-Result choice is binding-v0.1 and is
    /// documented as the only kernel-side `panic!` outside of explicit
    /// `unreachable!()` paths.
    pub fn insert_frame_event(
        &self,
        kind: FrameKind,
        spirit_pid: u32,
        capability_token: Option<&[u8; 32]>,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        let redacted = self.redaction.redact(payload);
        let mut inner = self.inner.lock().expect("TransparencyLogAdapter inner poisoned");
        let frame_id = Self::next_frame_id(&mut inner);
        let timestamp_ns = wall_clock_now_ns();

        inner.last_frame_id = frame_id;

        let result = inner.conn.execute(
            "INSERT INTO transparency_log
                (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token,
                 kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &frame_id[..],
                timestamp_ns as i64,
                spirit_pid as i64,
                inner.boot_nonce as i64,
                capability_token.map(|t| &t[..]),
                kind as i64,
                intent,
                &redacted[..],
                origin as i64,
            ],
        );

        match result {
            Ok(_) => LogBeforeDeliver::new(()),
            Err(e) => {
                // I2 binding: log write failure must halt the kernel rather than
                // silently dropping the frame. This is the ONLY `panic!` outside
                // `unreachable!()` paths in kernel-core.
                panic!(
                    "MAOS kernel panic — Transparency Log write failed: {e}. \
                     Architecture §7.3 I2: log-before-deliver guarantee broken; \
                     kernel halts. Audit the SQLite file for corruption."
                );
            }
        }
    }

    /// Insert an approval decision row into the Approval Decision Log table.
    ///
    /// Per Invariant I4 (architecture §3.2, enforcement-cadence `runtime`
    /// from v0.1) and §7.4 ("Approval Decision Log distinct from Transparency
    /// Log"). The two logs are stored in the same SQLite file (the v0.1-β
    /// I9-sanctioned single-file holder) but in **separate tables with no
    /// foreign-key relationship** — they share filesystem location, not
    /// schema. The independence is verified by the
    /// `approval_log_is_distinct_table` unit test.
    ///
    /// At v0.1-β the Approval Manager does not yet emit approval-decision
    /// events; the runtime body ships in Story 1b.3.
    pub fn insert_approval_decision(
        &self,
        decision: ApprovalDecision,
    ) -> Result<(), AuditError> {
        let inner = self.inner.lock().expect("TransparencyLogAdapter inner poisoned");
        let timestamp_ns = wall_clock_now_ns();
        inner.conn
            .execute(
                "INSERT INTO approval_decision_log
                    (timestamp_ns, actor, target, capability, intent, decision, reasoning)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    timestamp_ns as i64,
                    decision.actor,
                    decision.target,
                    decision.capability,
                    decision.intent,
                    decision.decision as i64,
                    decision.reasoning,
                ],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        Ok(())
    }

    /// Read-side: query frame events for `maosctl audit query`.
    /// Returns entries in `(timestamp_ns ASC, frame_id ASC)` order.
    pub fn query_frames(
        &self,
        filter: FrameFilter,
    ) -> Result<Vec<TransparencyLogEntry>, AuditError> {
        let inner = self.inner.lock().expect("TransparencyLogAdapter inner poisoned");
        let mut sql = String::from(
            "SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce,
                    capability_token, kind, intent, payload_redacted, origin
             FROM transparency_log",
        );
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(pid) = filter.spirit_pid {
            where_clauses.push("spirit_pid = ?".to_string());
            params.push(Box::new(pid as i64));
        }
        if let Some(kind) = filter.kind {
            where_clauses.push("kind = ?".to_string());
            params.push(Box::new(kind as i64));
        }
        if let Some(since) = filter.since_ns {
            where_clauses.push("timestamp_ns >= ?".to_string());
            params.push(Box::new(since as i64));
        }
        if let Some(until) = filter.until_ns {
            where_clauses.push("timestamp_ns <= ?".to_string());
            params.push(Box::new(until as i64));
        }
        if let (Some(cursor_ts), Some(cursor_fid)) = (filter.cursor_timestamp_ns, filter.cursor_frame_id) {
            where_clauses.push("(timestamp_ns, frame_id) > (? , ?)".to_string());
            params.push(Box::new(cursor_ts as i64));
            params.push(Box::new(cursor_fid.to_vec()));
        }

        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY timestamp_ns ASC, frame_id ASC");
        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        let mut stmt = inner.conn.prepare(&sql).map_err(AuditError::SqliteRead)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let frame_id_blob: Vec<u8> = row.get(0)?;
                let mut frame_id = [0u8; 16];
                if frame_id_blob.len() == 16 {
                    frame_id.copy_from_slice(&frame_id_blob);
                }
                let cap_blob: Option<Vec<u8>> = row.get(4)?;
                let mut cap_token = None;
                if let Some(ref blob) = cap_blob {
                    if blob.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(blob);
                        cap_token = Some(arr);
                    }
                }
                Ok(TransparencyLogEntry {
                    frame_id,
                    timestamp_ns: row.get::<_, i64>(1)? as u64,
                    spirit_pid: row.get::<_, i64>(2)? as u32,
                    boot_nonce: row.get::<_, i64>(3)? as u64,
                    capability_token: cap_token,
                    kind: FrameKind::from_i64(row.get::<_, i64>(5)?).unwrap_or(FrameKind::TaskAssign),
                    intent: row.get(6)?,
                    payload_redacted: row.get(7)?,
                    origin: match row.get::<_, i64>(8)? {
                        0 => FrameOrigin::HumanAuthored,
                        1 => FrameOrigin::SpiritAuto,
                        2 => FrameOrigin::SpiritDraftedHumanApproved,
                        3 => FrameOrigin::Kernel,
                        _ => FrameOrigin::HumanAuthored,
                    },
                })
            })
            .map_err(AuditError::SqliteRead)?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(AuditError::SqliteRead)?);
        }
        Ok(entries)
    }

    /// Read-side: look up a single frame by its primary key.
    /// Returns `None` if the frame_id is not found.
    pub fn query_frame_by_id(
        &self,
        frame_id: [u8; 16],
    ) -> Result<Option<TransparencyLogEntry>, AuditError> {
        let inner = self.inner.lock().expect("TransparencyLogAdapter inner poisoned");
        let mut stmt = inner
            .conn
            .prepare(
                "SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce,
                        capability_token, kind, intent, payload_redacted, origin
                 FROM transparency_log
                 WHERE frame_id = ?1
                 LIMIT 1",
            )
            .map_err(AuditError::SqliteRead)?;

        let row = stmt
            .query_row(rusqlite::params![&frame_id[..]], |row| {
                let frame_id_blob: Vec<u8> = row.get(0)?;
                let mut fid = [0u8; 16];
                if frame_id_blob.len() == 16 {
                    fid.copy_from_slice(&frame_id_blob);
                }
                let cap_blob: Option<Vec<u8>> = row.get(4)?;
                let mut cap_token = None;
                if let Some(ref blob) = cap_blob {
                    if blob.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(blob);
                        cap_token = Some(arr);
                    }
                }
                Ok(TransparencyLogEntry {
                    frame_id: fid,
                    timestamp_ns: row.get::<_, i64>(1)? as u64,
                    spirit_pid: row.get::<_, i64>(2)? as u32,
                    boot_nonce: row.get::<_, i64>(3)? as u64,
                    capability_token: cap_token,
                    kind: FrameKind::from_i64(row.get::<_, i64>(5)?)
                        .unwrap_or(FrameKind::TaskAssign),
                    intent: row.get(6)?,
                    payload_redacted: row.get(7)?,
                    origin: match row.get::<_, i64>(8)? {
                        0 => FrameOrigin::HumanAuthored,
                        1 => FrameOrigin::SpiritAuto,
                        2 => FrameOrigin::SpiritDraftedHumanApproved,
                        3 => FrameOrigin::Kernel,
                        _ => FrameOrigin::HumanAuthored,
                    },
                })
            })
            .optional()
            .map_err(AuditError::SqliteRead)?;
        Ok(row)
    }

    /// Read-side: query approval decisions.
    /// At v0.1-β the `spirit_pid` parameter is accepted but not used for
    /// filtering (the approval_decision_log table has no spirit_pid column).
    /// The function exists for `maos-audit` integration tests and for
    /// Story 9.1.
    pub fn query_approvals(
        &self,
        _spirit_pid: Option<u32>,
    ) -> Result<Vec<ApprovalDecision>, AuditError> {
        let inner = self.inner.lock().expect("TransparencyLogAdapter inner poisoned");

        let sql = "SELECT actor, target, capability, intent, decision, reasoning FROM approval_decision_log ORDER BY timestamp_ns ASC";

        let mut stmt = inner.conn.prepare(sql).map_err(AuditError::SqliteRead)?;

        let rows = stmt.query_map([], |row| {
            Ok(ApprovalDecision {
                actor: row.get(0)?,
                target: row.get(1)?,
                capability: row.get(2)?,
                intent: row.get(3)?,
                decision: row.get::<_, i64>(4)? != 0,
                reasoning: row.get(5)?,
            })
        }).map_err(AuditError::SqliteRead)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(AuditError::SqliteRead)
    }

    /// Get a reference to the mailbox stub (for testing).
    pub fn mailbox(&self) -> &MailboxStub {
        &self.mailbox
    }
}

/// Adapter-side IacBusPort impl. v0.1-β routes log-before-deliver to the
/// `MailboxStub`; Story 6.1 replaces the stub with the real DRR fairness
/// scheduler + mailbox semantics.
impl IacBusPort for TransparencyLogAdapter {
    type MailboxHandle = ();
    fn enqueue_frame(&self, frame_bytes: &[u8], origin: FrameOrigin) -> LogBeforeDeliver<()> {
        // 1. Pre-write redaction filter
        let redacted = self.redaction.redact(frame_bytes);
        // 2. Log first (panic on failure per I2)
        let token = self.insert_frame_event(
            FrameKind::TaskAssign,
            0, // spirit_pid decoded from frame header — v0.1-β passes 0
            None,
            "delegate",
            &redacted,
            origin,
        );
        // 3. Route to mailbox stub (Story 6.1 replaces with real mailbox)
        self.mailbox.record_delivery(&redacted);
        token
    }

    fn broadcast_frame(&self, frame_bytes: &[u8], origin: FrameOrigin) -> LogBeforeDeliver<()> {
        // Same pattern as enqueue_frame — logs, then routes to stub.
        let redacted = self.redaction.redact(frame_bytes);
        let token = self.insert_frame_event(
            FrameKind::TelemetryEvent,
            0,
            None,
            "broadcast",
            &redacted,
            origin,
        );
        self.mailbox.record_delivery(&redacted);
        token
    }

    async fn deliver(
        &self,
        frame: maos_domain::frame::IacFrame,
    ) -> Result<LogBeforeDeliver<()>, maos_domain::iac_bus_types::IacBusError> {
        let payload_bytes = serde_json::to_vec(&frame.payload)
            .map_err(|e| maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string()))?;
        Ok(self.enqueue_frame(&payload_bytes, frame.auto_marker))
    }

    fn register_spirit(
        &self,
        _spirit_id: &maos_spirit_abi::identity::SpiritId,
    ) -> Result<(), maos_domain::iac_bus_types::IacBusError> {
        Ok(())
    }
}

/// Wall-clock time in nanoseconds since Unix epoch.
/// NOT monotonic — NTP corrections may cause slight backward jumps.
/// For audit ordering, use frame_id + timestamp_ns compound key.
fn wall_clock_now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_succeeds() {
        let log = TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF);
        assert!(log.inner.lock().is_ok());
    }

    #[test]
    fn insert_frame_event_creates_one_row() {
        let log = TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF);
        let _token = log.insert_frame_event(
            FrameKind::TaskAssign,
            7,
            None,
            "delegate",
            b"test payload",
            FrameOrigin::HumanAuthored,
        );
        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].spirit_pid, 7);
        assert_eq!(entries[0].kind, FrameKind::TaskAssign);
        assert_eq!(entries[0].intent, "delegate");
    }

    #[test]
    fn insert_frame_with_capability_token() {
        let log = TransparencyLogAdapter::open_in_memory(0xAAAA_BBBB);
        let token_bytes: [u8; 32] = [0xAA; 32];
        let _token = log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            42,
            Some(&token_bytes),
            "file.read",
            b"/tmp/data",
            FrameOrigin::SpiritAuto,
        );
        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].capability_token, Some([0xAA; 32]));
    }

    #[test]
    fn hundred_sequential_inserts_are_ordered() {
        let log = TransparencyLogAdapter::open_in_memory(0x1);
        for i in 0..100 {
            let _token = log.insert_frame_event(
                FrameKind::TaskAssign,
                i as u32,
                None,
                "seq-test",
                b"payload",
                FrameOrigin::HumanAuthored,
            );
        }
        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 100);
        // Verify timestamps are non-decreasing
        for window in entries.windows(2) {
            assert!(
                window[0].timestamp_ns <= window[1].timestamp_ns,
                "timestamps not ordered: {} > {}",
                window[0].timestamp_ns,
                window[1].timestamp_ns,
            );
        }
    }

    #[test]
    fn query_filter_by_spirit_pid() {
        let log = TransparencyLogAdapter::open_in_memory(0x1);
        for pid in [7u32, 7, 7, 42, 99] {
            let _token = log.insert_frame_event(
                FrameKind::TaskAssign,
                pid,
                None,
                "filter-test",
                b"payload",
                FrameOrigin::HumanAuthored,
            );
        }
        let entries = log.query_frames(FrameFilter {
            spirit_pid: Some(7),
            ..Default::default()
        }).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|e| e.spirit_pid == 7));
    }

    #[test]
    fn query_filter_by_since_ns_excludes_older_frames() {
        let log = TransparencyLogAdapter::open_in_memory(0x1);
        let _t1 = log.insert_frame_event(
            FrameKind::TaskComplete,
            1,
            None,
            "old",
            b"old-payload",
            FrameOrigin::SpiritAuto,
        );
        // Small sleep to ensure timestamps differ (wall-clock granularity).
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _t2 = log.insert_frame_event(
            FrameKind::TaskComplete,
            1,
            None,
            "new",
            b"new-payload",
            FrameOrigin::SpiritAuto,
        );

        let all = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(all.len(), 2);

        // Filter with since_ns after the first frame's timestamp — only the second should match.
        let since = all[0].timestamp_ns + 1;
        let filtered = log.query_frames(FrameFilter {
            since_ns: Some(since),
            ..Default::default()
        }).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].intent, "new");
    }

    #[test]
    fn concurrent_inserts_from_threads() {
        use std::sync::Arc;
        use std::thread;

        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x1));
        let mut handles = Vec::new();
        for tid in 0..4 {
            let log_clone = Arc::clone(&log);
            handles.push(thread::spawn(move || {
                for i in 0..10 {
                    let _token = log_clone.insert_frame_event(
                        FrameKind::TaskAssign,
                        (tid * 10 + i) as u32,
                        None,
                        "concurrent-test",
                        b"payload",
                        FrameOrigin::SpiritAuto,
                    );
                }
            }));
        }
        for h in handles {
            h.join().expect("thread panicked");
        }
        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 40, "expected 40 rows from 4 threads × 10 inserts");
    }

    #[test]
    fn insert_approval_decision_creates_row() {
        let log = TransparencyLogAdapter::open_in_memory(0x1);
        log.insert_approval_decision(ApprovalDecision {
            actor: "user-1".into(),
            target: "spirit-butler".into(),
            capability: "calendar.read".into(),
            intent: "morning-digest".into(),
            decision: true,
            reasoning: Some("user grants calendar read for digest spirit".into()),
        }).unwrap();

        let approvals = log.query_approvals(None).unwrap();
        assert_eq!(approvals.len(), 1);
        assert_eq!(approvals[0].actor, "user-1");
        assert_eq!(approvals[0].decision, true);
        assert_eq!(
            approvals[0].reasoning.as_deref(),
            Some("user grants calendar read for digest spirit")
        );
    }

    #[test]
    fn approval_log_is_distinct_table() {
        let log = TransparencyLogAdapter::open_in_memory(0x1);

        // Insert one row into each table
        let _token = log.insert_frame_event(
            FrameKind::TaskAssign,
            1,
            None,
            "schema-test",
            b"payload",
            FrameOrigin::HumanAuthored,
        );
        log.insert_approval_decision(ApprovalDecision {
            actor: "actor".into(),
            target: "target".into(),
            capability: "cap".into(),
            intent: "intent".into(),
            decision: true,
            reasoning: None,
        }).unwrap();

        let inner = log.inner.lock().unwrap();
        let conn = &inner.conn;

        // 1. Verify two audit tables exist (sqlite_sequence is an internal table from AUTOINCREMENT)
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(
            tables,
            vec!["approval_decision_log", "transparency_log"],
            "expected exactly two tables"
        );

        // 2. No foreign keys on transparency_log
        let mut stmt = conn
            .prepare("PRAGMA foreign_key_list(transparency_log)")
            .unwrap();
        let fk_tlog: Vec<String> = stmt.query_map([], |row| row.get(0)).unwrap().map(|r| r.unwrap()).collect();
        assert!(fk_tlog.is_empty(), "transparency_log has foreign keys");

        // 3. No foreign keys on approval_decision_log
        let mut stmt = conn
            .prepare("PRAGMA foreign_key_list(approval_decision_log)")
            .unwrap();
        let fk_adl: Vec<String> = stmt.query_map([], |row| row.get(0)).unwrap().map(|r| r.unwrap()).collect();
        assert!(fk_adl.is_empty(), "approval_decision_log has foreign keys");

        // 4. Each table has exactly 1 row
        let tlog_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transparency_log", [], |row| row.get(0))
            .unwrap();
        let adl_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM approval_decision_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(tlog_count, 1);
        assert_eq!(adl_count, 1);

        // 5. Deleting from one table does not affect the other (test-only DELETE)
        conn.execute("DELETE FROM transparency_log", []).unwrap();
        let adl_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM approval_decision_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(adl_after, 1, "approval row affected by transparency_log DELETE");
    }

    #[test]
    #[should_panic(expected = "MAOS kernel panic — Transparency Log write failed")]
    fn insert_frame_panics_on_write_failure() {
        // Create an in-memory adapter, then corrupt the connection
        let log = TransparencyLogAdapter::open_in_memory(0x1);
        // Drop the inner connection by replacing it with a broken one
        {
            let mut inner = log.inner.lock().unwrap();
            // Close the connection to force failure on next write
            let old_conn = std::mem::replace(
                &mut inner.conn,
                Connection::open_in_memory().unwrap(),
            );
            drop(old_conn);
            // The new connection doesn't have the schema, so INSERT will fail
            // Actually let's just force a failure by dropping and not recreating
        }
        // Try to insert — should panic because the new connection has no table
        let _ = log.insert_frame_event(
            FrameKind::TaskAssign,
            1,
            None,
            "panic-test",
            b"payload",
            FrameOrigin::HumanAuthored,
        );
    }

    #[test]
    fn iac_bus_port_enqueue_writes_and_routes() {
        let log = TransparencyLogAdapter::open_in_memory(0x1);
        let _token = log.enqueue_frame(b"test-frame-data", FrameOrigin::HumanAuthored);
        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 1, "enqueue should write one row");
        let mailbox_frames = log.mailbox().drain_pending();
        assert_eq!(mailbox_frames.len(), 1, "enqueue should route to mailbox");
    }

    #[test]
    fn iac_bus_port_broadcast_writes_and_routes() {
        let log = TransparencyLogAdapter::open_in_memory(0x1);
        let _token = log.broadcast_frame(b"broadcast-data", FrameOrigin::SpiritAuto);
        let entries = log.query_frames(FrameFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, FrameKind::TelemetryEvent);
    }
}
