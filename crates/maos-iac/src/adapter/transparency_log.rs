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
//! This file is the historical holder for the SQLite connection and in-memory
//! frame_id counter. The previously documented `#[i9_exempt]` attribute is not
//! present, the old whitelist path is stale, and `check-empty-kernel` does not
//! scan this crate. Story 13.5e records that enforcement gap rather than
//! treating prose as an active control; lint repair remains separately filed.
//!
//! # Shared multi-writer contract (Story 9.7 AC-7)
//!
//! The TL is a **shared insert-only multi-writer log**.  Both the daemon
//! (`maos-bin`) and the CLI (`maosctl skills approve/reject`) may write
//! concurrently:
//!
//! - **WAL mode** — set at `open_with_policy` time.
//! - **`busy_timeout = 5000`** — a second writer blocks up to 5 s rather
//!   than failing immediately with `SQLITE_BUSY`.
//! - **Append-only** — no cross-row updates; each decision is a new row.
//! - **`ORDER BY timestamp_ns ASC, decision_id ASC`** — tie-break on the
//!   autoincrement `decision_id` guards against NTP step-backward.
//!
//! The residual after-timeout race is an accepted limitation; the true
//! retirement is routing the CLI write through the daemon (one writer) —
//! filed as an Epic-10 follow-up.

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
#[non_exhaustive]
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
    /// Story 5.2 — Hot-swap aborted (swap-out failed, swap-in failed, or auto-revert).
    HotSwapAborted = 14,
    /// Story 5.3 — Spirit holding in-flight task emitted no progress IAC for > threshold.
    TaskStalled = 15,
    /// Story 5.3 — Spirit emitted heartbeat but no progress IAC for > threshold.
    SilentFailureSuspect = 16,
    /// Story 5.4 — Spirit was revoked via CRL propagation.
    SpiritRevoked = 17,
    /// Story 5.5c — MCP tool invocation routed through kernel capability mediation.
    McpInvocation = 18,
    /// Story 5.5d — Spirit admitted to the kernel via registry.
    SpiritAdmitted = 19,
    /// Story 5.5d — Registry yank propagated to the kernel.
    RegistryYank = 20,
    /// Story 6.2 AC6 — FR52: a line of stdout/stderr captured from a
    /// CliWrapperSpirit's invoked CLI subprocess.
    CliSubprocessOutput = 21,
    /// Story 6.4 — ADR-034 binding-v0.9: partial-consent rupture event.
    ConsentRupture = 22,
    /// Story 6.4 — NFR-Scale-4: per-(provider, credential) bucket-exhaustion event.
    RateLimited = 23,
    /// Story 6.5 — FR54: inbound message from external gateway.
    GatewayInbound = 24,
    /// Story 6.5 — FR54: outbound message to external gateway.
    GatewayOutbound = 25,
    /// Story 7.2 — FR60: Spirit admitted via air-gapped `maosctl import --offline`.
    /// Distinguishable in audit from registry-served admissions
    /// (`SpiritAdmitted = 19`) and CRL revocations (`SpiritRevoked = 17`).
    SpiritImported = 26,
    /// Story 7.4 — FR40 "full": a CliWrapperSpirit admission was REFUSED because
    /// the observed CLI output shape did not match the declared
    /// `output_shape_version` (ADR-021 `EOutputShapeAdapterMismatch`). The
    /// payload carries `{cli, declared, observed}` so the refusal is auditable.
    /// The probe logic is UNCHANGED (Story 6.2); this row makes the refusal —
    /// previously a returned-but-unjournaled error — appear in the Transparency
    /// Log before the kernel returns, per ADR-021's "audit drift is the failure
    /// mode the substrate cannot tolerate".
    CliWrapperShapeMismatch = 27,
    /// Story 9.3b — FR62 (ADR-045): governance audit artifact.  Payload is
    /// `GovernanceEventPayload` (discriminated by `GovernanceEventKind`):
    /// ABI-extension proposals/ratification, vetter-key admission/rotation,
    /// ComplianceClaim schema-lifecycle events.
    GovernanceEvent = 28,
    /// Story 9.3b — FR64 (ADR-046): cost-attribution fact.  Payload is
    /// `CostAttributionPayload` (RAW dimensional facts — no money field;
    /// money computed read-time in `maos-audit` via `ProviderPricingConfig`).
    CostAttribution = 29,
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
            14 => Some(Self::HotSwapAborted),
            15 => Some(Self::TaskStalled),
            16 => Some(Self::SilentFailureSuspect),
            17 => Some(Self::SpiritRevoked),
            18 => Some(Self::McpInvocation),
            19 => Some(Self::SpiritAdmitted),
            20 => Some(Self::RegistryYank),
            21 => Some(Self::CliSubprocessOutput),
            22 => Some(Self::ConsentRupture),
            23 => Some(Self::RateLimited),
            24 => Some(Self::GatewayInbound),
            25 => Some(Self::GatewayOutbound),
            26 => Some(Self::SpiritImported),
            27 => Some(Self::CliWrapperShapeMismatch),
            28 => Some(Self::GovernanceEvent),
            29 => Some(Self::CostAttribution),
            _ => None,
        }
    }
}

/// Capability token proving the caller is the `DistillateWriter` path
/// (Story 8.10 AC2 — I11 citer-authorization gate).
///
/// Constructable only within `maos-iac` (`pub(crate)` ctor), so a
/// `FrameKind::Distillate` row can **only** be inserted via
/// [`TransparencyLogAdapter::insert_distillate_frame`], which the
/// `DistillateWriter` holds. The public `insert_frame_event*` paths
/// reject `FrameKind::Distillate` outright. Together these make the I11
/// audit-chain validation in `write_distillate` unbypassable at runtime
/// (previously "forbidden by convention" with zero enforcement).
#[derive(Debug)]
pub struct DistillateWriteToken(());

impl DistillateWriteToken {
    /// Mint the token. `pub(crate)` so only `maos-iac` internals (the
    /// `DistillateWriter`) can authorize a `Distillate` insert.
    pub(crate) fn new() -> Self {
        Self(())
    }
}

/// A single Transparency Log row — what `query_frames` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparencyLogEntry {
    pub frame_id: [u8; 16], // ULID bytes
    pub timestamp_ns: u64,
    pub spirit_pid: u32,
    pub from_spirit_id: String,
    pub to_spirit_id: String,
    pub boot_nonce: u64,
    pub capability_token: Option<[u8; 32]>,
    pub kind: FrameKind,
    pub intent: String,
    pub correlation_id: Option<String>,
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
    pub correlation_id: Option<String>,
    pub since_ns: Option<u64>,
    pub until_ns: Option<u64>,
    pub limit: Option<usize>,
    /// Story 6.1 — filter by exact frame_id (for retract authority lookup).
    pub frame_id: Option<[u8; 16]>,
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
    #[error("malformed frame_id blob: expected 16 bytes, got {0}")]
    MalformedFrameId(usize),
    #[error("serialization failed: {0}")]
    Serialization(String),
}

/// SQL schema for both tables.
const SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    timestamp_ns        INTEGER NOT NULL,
    spirit_pid          INTEGER NOT NULL,
    from_spirit_id      TEXT    NOT NULL DEFAULT '',
    to_spirit_id        TEXT    NOT NULL DEFAULT '',
    boot_nonce          INTEGER NOT NULL,
    capability_token    BLOB,
    kind                INTEGER NOT NULL,
    intent              TEXT    NOT NULL,
    correlation_id      TEXT,
    payload_redacted    BLOB    NOT NULL,
    origin              INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tlog_spirit_pid
    ON transparency_log(spirit_pid, timestamp_ns);
CREATE INDEX IF NOT EXISTS idx_tlog_kind
    ON transparency_log(kind, timestamp_ns);

-- Story 6.1 — retraction companion table.
-- Per AC2: original row is APPEND-ONLY-PRESERVED.
-- The retraction marker lives in this companion table, not as a column
-- on transparency_log, to minimize schema migration blast radius.
CREATE TABLE IF NOT EXISTS transparency_log_retractions (
    original_frame_id   BLOB    NOT NULL PRIMARY KEY,
    retract_frame_id    BLOB    NOT NULL,
    retracted_at_ns     INTEGER NOT NULL
);

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

-- Story 9.2 (P29) — durable per-principal-global legal holds.  Consulted by
-- every forget/uninstall so a held principal cannot be erased by a later
-- command until explicitly released (Decision E: release re-queues, never
-- auto-fires). In team mode Story 13.5e attaches the Host-global table to the
-- one team TL connection and removes the shard-local compatibility table.
CREATE TABLE IF NOT EXISTS legal_holds (
    principal_id        TEXT    NOT NULL PRIMARY KEY,
    reason              TEXT    NOT NULL,
    case_ref            TEXT,
    requested_at_ns     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_approval_actor
    ON approval_decision_log(actor, timestamp_ns);

-- Story 9.3b (R10) — schema-lifecycle registry for governance audit.
CREATE TABLE IF NOT EXISTS schema_lifecycle_registry (
    schema_id           TEXT    NOT NULL,
    version             INTEGER NOT NULL,
    effective_at_ns     INTEGER NOT NULL,
    supersedes_hash     TEXT,
    ratified_by         TEXT    NOT NULL,
    recorded_at_ns      INTEGER NOT NULL,
    schema_content_hash TEXT    NOT NULL,
    PRIMARY KEY (schema_id, version)
);

CREATE INDEX IF NOT EXISTS idx_slr_schema_id
    ON schema_lifecycle_registry(schema_id);
CREATE INDEX IF NOT EXISTS idx_slr_version
    ON schema_lifecycle_registry(version);
CREATE INDEX IF NOT EXISTS idx_slr_recorded_at
    ON schema_lifecycle_registry(recorded_at_ns);
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
        self.inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned")
            .last_frame_id
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
        Self::open_with_policy_and_timeout(path, boot_nonce, redaction, 5000)
    }

    /// Open with a custom redaction policy AND a configurable `busy_timeout`
    /// (ms). The 5000 ms default (above) is the documented multi-writer
    /// ceiling; this entry point lets the AC-7 contention contract be proven in
    /// BOTH directions — `busy_timeout=0` RED's immediately under contention,
    /// `busy_timeout=5000` blocks-then-succeeds (GREEN). (Story 9.7 #8.)
    #[doc(hidden)]
    pub fn open_with_policy_and_timeout(
        path: &Path,
        boot_nonce: u64,
        redaction: Box<dyn RedactionPolicy + Send + Sync>,
        busy_timeout_ms: u64,
    ) -> Result<Self, AuditError> {
        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::default() | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        // Story 9.7 R3 — multi-writer SQLite contract: the CLI may write to
        // the TL concurrently with the daemon. busy_timeout lets the second
        // writer BLOCK-then-succeed instead of failing immediately with
        // SQLITE_BUSY; the residual true-race after the timeout is the only
        // accepted limitation.
        conn.busy_timeout(std::time::Duration::from_millis(busy_timeout_ms))?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(SCHEMA_SQL)?;
        // Story 6.1 — migration: add to_spirit_id column to existing databases
        let _ = conn.execute_batch(
            "ALTER TABLE transparency_log ADD COLUMN to_spirit_id TEXT NOT NULL DEFAULT '';",
        );
        let _ = conn.execute_batch(
            "ALTER TABLE transparency_log ADD COLUMN from_spirit_id TEXT NOT NULL DEFAULT '';",
        );
        let _ = conn.execute_batch("ALTER TABLE transparency_log ADD COLUMN correlation_id TEXT;");
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_tlog_correlation
             ON transparency_log(correlation_id, timestamp_ns);",
        )?;
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

    /// Open with the default redaction policy and a configurable `busy_timeout`
    /// (ms) — test entry point for the AC-7 contention contract.
    #[doc(hidden)]
    pub fn open_with_busy_timeout(
        path: &Path,
        boot_nonce: u64,
        busy_timeout_ms: u64,
    ) -> Result<Self, AuditError> {
        Self::open_with_policy_and_timeout(
            path,
            boot_nonce,
            Box::new(CorpusBackedRedactionPolicy::new()),
            busy_timeout_ms,
        )
    }

    /// Open an in-memory SQLite database for tests.
    #[doc(hidden)]
    pub fn open_in_memory(boot_nonce: u64) -> Self {
        Self::open_in_memory_with_policy(boot_nonce, Box::new(CorpusBackedRedactionPolicy::new()))
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

    /// Backward-compatible wrapper — delegates to [`Self::insert_frame_event_with_sender`]
    /// with empty sender/recipient IDs. Used by callers that do not need
    /// retraction authority tracking.
    pub fn insert_frame_event(
        &self,
        kind: FrameKind,
        spirit_pid: u32,
        capability_token: Option<&[u8; 32]>,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        self.insert_frame_event_with_sender(
            kind,
            spirit_pid,
            "",
            "",
            capability_token,
            intent,
            payload,
            origin,
        )
    }

    /// Insert a frame event that participates in a cross-team audit action.
    ///
    /// Existing kernel callers remain on [`Self::insert_frame_event`]; only
    /// out-of-kernel composition paths that already own a correlation token use
    /// this additive writer.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_frame_event_with_correlation(
        &self,
        kind: FrameKind,
        spirit_pid: u32,
        capability_token: Option<&[u8; 32]>,
        correlation_id: &str,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        if kind == FrameKind::Distillate {
            panic!(
                "MAOS I11 enforcement (Story 8.10 AC2): FrameKind::Distillate rows \
                 may only be inserted via DistillateWriter"
            );
        }
        self.insert_frame_row_with_correlation(
            None,
            kind,
            spirit_pid,
            "",
            "",
            capability_token,
            Some(correlation_id),
            intent,
            payload,
            origin,
        )
    }

    /// Insert a frame event with sender tracking. Returns `LogBeforeDeliver<()>` per I2 typestate:
    /// the caller can only construct `LogBeforeDeliver` by going through
    /// this method (the `i2::LogBeforeDeliver::new` constructor uses
    /// `#[doc(hidden)] pub` — visible but convention-gated at v0.1-beta).
    ///
    /// On SQLite write failure: PANICS per architecture §7.3 I2 ("if the
    /// log write fails, the kernel panics rather than silently dropping
    /// the frame"). The panic-vs-Result choice is binding-v0.1 and is
    /// documented as the only kernel-side `panic!` outside of explicit
    /// `unreachable!()` paths.
    pub fn insert_frame_event_with_sender(
        &self,
        kind: FrameKind,
        spirit_pid: u32,
        from_spirit_id: &str,
        to_spirit_id: &str,
        capability_token: Option<&[u8; 32]>,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        self.insert_frame_event_with_id(
            None,
            kind,
            spirit_pid,
            from_spirit_id,
            to_spirit_id,
            capability_token,
            intent,
            payload,
            origin,
        )
    }

    /// Insert a frame event with explicit frame ID and sender tracking.
    ///
    /// Use this when the frame ID is externally generated (e.g. an IAC frame
    /// that must be retrievable by its original frame_id for retraction).
    /// Pass `None` for `frame_id` to auto-generate one.
    pub fn insert_frame_event_with_id(
        &self,
        frame_id: Option<[u8; 16]>,
        kind: FrameKind,
        spirit_pid: u32,
        from_spirit_id: &str,
        to_spirit_id: &str,
        capability_token: Option<&[u8; 32]>,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        // Story 8.10 AC2 (I11 citer-authorization gate): a `Distillate` row may
        // ONLY be inserted via `insert_distillate_frame` (which the
        // `DistillateWriter` holds the `DistillateWriteToken` for). A direct
        // public insert of a `Distillate` kind would bypass the I11 audit-chain
        // validation + citer-auth check — forbidden at runtime, not just by
        // convention. This is an enforcement panic in the I2 family.
        if kind == FrameKind::Distillate {
            panic!(
                "MAOS I11 enforcement (Story 8.10 AC2): FrameKind::Distillate rows \
                 may only be inserted via DistillateWriter (insert_distillate_frame); \
                 a direct insert_frame_event(FrameKind::Distillate, …) bypasses the \
                 I11 audit-chain + citer-authorization checks and is forbidden."
            );
        }
        self.insert_frame_row(
            frame_id,
            kind,
            spirit_pid,
            from_spirit_id,
            to_spirit_id,
            capability_token,
            intent,
            payload,
            origin,
        )
    }

    /// Token-guarded `Distillate` inserter (Story 8.10 AC2). The ONLY path that
    /// may write a `FrameKind::Distillate` row. The [`DistillateWriteToken`] is
    /// `pub(crate)`-constructable, so only the `DistillateWriter` inside
    /// `maos-iac` can reach this; external code paths cannot forge a token and
    /// the public `insert_frame_event*` paths reject `Distillate`.
    pub fn insert_distillate_frame(
        &self,
        _token: DistillateWriteToken,
        frame_id: Option<[u8; 16]>,
        spirit_pid: u32,
        from_spirit_id: &str,
        to_spirit_id: &str,
        capability_token: Option<&[u8; 32]>,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        self.insert_frame_row(
            frame_id,
            FrameKind::Distillate,
            spirit_pid,
            from_spirit_id,
            to_spirit_id,
            capability_token,
            intent,
            payload,
            origin,
        )
    }

    /// Internal row writer (no FrameKind gating). Shared by the public
    /// `insert_frame_event*` wrappers and the token-guarded distillate path.
    #[allow(clippy::too_many_arguments)]
    fn insert_frame_row(
        &self,
        frame_id: Option<[u8; 16]>,
        kind: FrameKind,
        spirit_pid: u32,
        from_spirit_id: &str,
        to_spirit_id: &str,
        capability_token: Option<&[u8; 32]>,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        self.insert_frame_row_with_correlation(
            frame_id,
            kind,
            spirit_pid,
            from_spirit_id,
            to_spirit_id,
            capability_token,
            None,
            intent,
            payload,
            origin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_frame_row_with_correlation(
        &self,
        frame_id: Option<[u8; 16]>,
        kind: FrameKind,
        spirit_pid: u32,
        from_spirit_id: &str,
        to_spirit_id: &str,
        capability_token: Option<&[u8; 32]>,
        correlation_id: Option<&str>,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
    ) -> LogBeforeDeliver<()> {
        let redacted = self.redaction.redact(payload);
        let mut inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let frame_id = frame_id.unwrap_or_else(|| Self::next_frame_id(&mut inner));
        let timestamp_ns = wall_clock_now_ns();

        inner.last_frame_id = frame_id;

        let result = inner.conn.execute(
            "INSERT INTO transparency_log
                (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce,
                 capability_token, kind, intent, correlation_id, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                &frame_id[..],
                timestamp_ns as i64,
                spirit_pid as i64,
                from_spirit_id,
                to_spirit_id,
                inner.boot_nonce as i64,
                capability_token.map(|t| &t[..]),
                kind as i64,
                intent,
                correlation_id,
                &redacted[..],
                origin as i64,
            ],
        );

        match result {
            Ok(_) => LogBeforeDeliver::new(()),
            Err(e) => {
                panic!(
                    "MAOS kernel panic — Transparency Log write failed: {e}. \
                     Architecture §7.3 I2: log-before-deliver guarantee broken; \
                     kernel halts. Audit the SQLite file for corruption."
                );
            }
        }
    }
    /// Story 9.2 — overwrite a Distillate frame's payload with a redaction
    /// tombstone.  This is the body-scrub half of Decision C; the marker
    /// frame is appended separately so the audit chain remains append-only.
    pub fn scrub_distillate_body(
        &self,
        frame_id: [u8; 16],
        reason: &str,
    ) -> Result<(), AuditError> {
        let tombstone = serde_json::json!({
            "redacted": true,
            "reason": reason,
            "original_kind": "Distillate",
        });
        let mut inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let changed = inner
            .conn
            .execute(
                "UPDATE transparency_log SET payload_redacted = ?1 WHERE frame_id = ?2",
                rusqlite::params![tombstone.to_string().as_bytes(), &frame_id[..]],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        if changed == 0 {
            // P25: a scrub that matched no row is indistinguishable from
            // success unless we surface it — the distillate frame may not exist.
            return Err(AuditError::SqliteRead(rusqlite::Error::QueryReturnedNoRows));
        }
        Ok(())
    }

    /// Story 9.2 — append a redaction-marker frame referencing a Distillate
    /// frame.  Uses an intent string (`distillate.redacted`) rather than a new
    /// `FrameKind` to keep the ABI frozen.
    pub fn insert_distillate_redaction_marker(
        &self,
        principal_id: &str,
        distillate_frame_id: [u8; 16],
    ) -> LogBeforeDeliver<()> {
        let payload = serde_json::json!({
            "principal_id": principal_id,
            "redacted_distillate_frame_id": format_frame_id_hex(&distillate_frame_id),
        });
        self.insert_frame_event(
            FrameKind::TaskComplete,
            0,
            None,
            "distillate.redacted",
            payload.to_string().as_bytes(),
            FrameOrigin::Kernel,
        )
    }

    /// Story 9.2 — return every frame_id in the Transparency Log, sorted.
    pub fn all_frame_ids(&self) -> Result<Vec<[u8; 16]>, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let mut stmt = inner
            .conn
            .prepare(
                "SELECT frame_id FROM transparency_log ORDER BY timestamp_ns ASC, frame_id ASC",
            )
            .map_err(AuditError::SqliteRead)?;
        let rows = stmt
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                // P8: a corrupt/non-16-byte row must not panic the cascade.
                if bytes.len() != 16 {
                    return Err(rusqlite::Error::FromSqlConversionFailure(
                        16,
                        rusqlite::types::Type::Blob,
                        format!(
                            "transparency_log frame_id is {} bytes, expected 16",
                            bytes.len()
                        )
                        .into(),
                    ));
                }
                let mut arr = [0u8; 16];
                arr.copy_from_slice(&bytes);
                Ok(arr)
            })
            .map_err(AuditError::SqliteRead)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(AuditError::SqliteRead)?);
        }
        Ok(out)
    }

    /// Story 9.2 — list distinct principal_ids written by a given spirit pid.
    pub fn principal_ids_for_spirit_pid(&self, spirit_pid: u32) -> Result<Vec<String>, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let mut stmt = inner
            .conn
            .prepare(
                "SELECT DISTINCT principal_id FROM principal_index WHERE writer_spirit_pid = ?1",
            )
            .map_err(AuditError::SqliteRead)?;
        let rows = stmt
            .query_map(rusqlite::params![spirit_pid as i64], |row| {
                let pid: String = row.get(0)?;
                Ok(pid)
            })
            .map_err(AuditError::SqliteRead)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(AuditError::SqliteRead)?);
        }
        Ok(out)
    }

    /// Story 9.2 — return every Distillate frame authored by a Spirit in
    /// `writer_spirit_pids`, with its `payload_redacted` body.  Used by the
    /// forget cascade to find candidate distillates for body-scrub (P3).
    /// The cascade then applies a content-based filter so only distillates
    /// that actually reference the forgotten principal are scrubbed.
    pub fn distillate_frames_for_pids(
        &self,
        writer_spirit_pids: &std::collections::HashSet<u32>,
    ) -> Result<Vec<([u8; 16], Vec<u8>)>, AuditError> {
        if writer_spirit_pids.is_empty() {
            return Ok(Vec::new());
        }
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let placeholders = (0..writer_spirit_pids.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT frame_id, payload_redacted FROM transparency_log \
             WHERE kind = ?1 AND spirit_pid IN ({placeholders}) \
             ORDER BY timestamp_ns ASC",
        );
        let mut stmt = inner.conn.prepare(&sql).map_err(AuditError::SqliteRead)?;
        let pid_params: Vec<Box<dyn rusqlite::types::ToSql>> = std::iter::once(Box::new(
            FrameKind::Distillate as i64,
        )
            as Box<dyn rusqlite::types::ToSql>)
        .chain(
            writer_spirit_pids
                .iter()
                .map(|p| Box::new(*p as i64) as Box<dyn rusqlite::types::ToSql>),
        )
        .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            pid_params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let fid_bytes: Vec<u8> = row.get(0)?;
                let payload: Vec<u8> = row.get(1)?;
                let mut arr = [0u8; 16];
                if fid_bytes.len() == 16 {
                    arr.copy_from_slice(&fid_bytes);
                }
                Ok((arr, payload))
            })
            .map_err(AuditError::SqliteRead)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(AuditError::SqliteRead)?);
        }
        Ok(out)
    }

    /// Story 9.3b (SR-3) — return every cost-attribution or governance frame
    /// authored by a Spirit in `writer_spirit_pids` that embeds a principal_id.
    /// Used by the forget cascade to find candidates for body-scrub.
    ///
    /// Unlike distillate frames (which require a content-based body scan),
    /// cost/governance frames carry principal_id as a STRUCTURED field,
    /// so discovery is a clean indexed query.
    pub fn principal_bearing_frames_for_pids(
        &self,
        writer_spirit_pids: &std::collections::HashSet<u32>,
    ) -> Result<Vec<([u8; 16], Vec<u8>)>, AuditError> {
        if writer_spirit_pids.is_empty() {
            return Ok(Vec::new());
        }
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let placeholders = (0..writer_spirit_pids.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        // FrameKind 28 = GovernanceEvent, 29 = CostAttribution
        let sql = format!(
            "SELECT frame_id, payload_redacted FROM transparency_log \
             WHERE kind IN (28, 29) AND spirit_pid IN ({placeholders}) \
             ORDER BY timestamp_ns ASC",
        );
        let mut stmt = inner.conn.prepare(&sql).map_err(AuditError::SqliteRead)?;
        let pids: Vec<i64> = writer_spirit_pids.iter().map(|&p| p as i64).collect();
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = pids
            .iter()
            .map(|p| p as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_ref), |row| {
                let fid: Vec<u8> = row.get(0)?;
                let body: Vec<u8> = row.get(1)?;
                Ok((fid, body))
            })
            .map_err(AuditError::SqliteRead)?;
        let mut out = Vec::new();
        for row in rows {
            let (fid_vec, body) = row.map_err(AuditError::SqliteRead)?;
            if fid_vec.len() != 16 {
                return Err(AuditError::MalformedFrameId(fid_vec.len()));
            }
            let mut fid = [0u8; 16];
            fid.copy_from_slice(&fid_vec);
            out.push((fid, body));
        }
        Ok(out)
    }

    /// Story 9.3b (SR-3) — scrub a principal-bearing cost/governance frame's
    /// payload by replacing the principal_id with a redaction tombstone.
    /// This mirrors `scrub_distillate_body` but for structured payloads.
    pub fn scrub_principal_bearing_frame(
        &self,
        frame_id: [u8; 16],
        reason: &str,
    ) -> Result<(), AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        // Try to parse the existing payload and redact only the principal_id field.
        let existing: Vec<u8> = inner
            .conn
            .query_row(
                "SELECT payload_redacted FROM transparency_log WHERE frame_id = ?1",
                rusqlite::params![&frame_id[..]],
                |row| row.get(0),
            )
            .map_err(AuditError::SqliteRead)?;
        let redacted_bytes = match serde_json::from_slice::<serde_json::Value>(&existing) {
            Ok(mut val) => {
                if let Some(obj) = val.as_object_mut() {
                    if obj.contains_key("principal_id") {
                        obj.insert(
                            "principal_id".to_string(),
                            serde_json::Value::String("[REDACTED]".to_string()),
                        );
                    }
                    // Also record redaction metadata
                    obj.insert("redacted".to_string(), serde_json::Value::Bool(true));
                    obj.insert(
                        "redaction_reason".to_string(),
                        serde_json::Value::String(reason.to_string()),
                    );
                }
                serde_json::to_vec(&val).map_err(|e| AuditError::Serialization(e.to_string()))?
            }
            Err(_) => {
                // Cannot parse payload — fall back to full replacement tombstone
                let tombstone = serde_json::json!({
                    "redacted": true,
                    "reason": reason,
                    "original_frame_id": format_frame_id_hex(&frame_id),
                });
                tombstone.to_string().into_bytes()
            }
        };
        inner
            .conn
            .execute(
                "UPDATE transparency_log SET payload_redacted = ?1 WHERE frame_id = ?2",
                rusqlite::params![&redacted_bytes[..], &frame_id[..]],
            )
            .map_err(|e| {
                panic!("MAOS kernel panic — principal-bearing frame scrub failed: {e}. I2.");
            })
            .unwrap();

        Ok(())
    }

    /// Keep the principal-global legal-hold table on the Host-global artifact
    /// while this adapter writes frame/approval rows to a team shard.
    ///
    /// One connection remains authoritative: SQLite resolves the unqualified
    /// `legal_holds` name from the attached global database after the
    /// shard-local compatibility table is removed.
    pub fn attach_global_legal_holds(&self, global_path: &Path) -> Result<(), AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        inner
            .conn
            .execute(
                "ATTACH DATABASE ?1 AS maos_host_global",
                rusqlite::params![global_path.to_string_lossy().as_ref()],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        inner
            .conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS maos_host_global.legal_holds (
                    principal_id TEXT NOT NULL PRIMARY KEY,
                    reason TEXT NOT NULL,
                    case_ref TEXT,
                    requested_at_ns INTEGER NOT NULL
                 );
                 DROP TABLE main.legal_holds;",
            )
            .map_err(AuditError::SqliteWriteFatal)
    }

    /// Story 9.2 (P29) — place a durable per-principal-global legal hold.
    /// Idempotent: re-placing replaces the reason/case_ref/timestamp.
    pub fn place_legal_hold(
        &self,
        principal_id: &str,
        reason: &str,
        case_ref: Option<&str>,
        requested_at_ns: u64,
    ) -> Result<(), AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        inner
            .conn
            .execute(
                "INSERT OR REPLACE INTO legal_holds \
                 (principal_id, reason, case_ref, requested_at_ns) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![principal_id, reason, case_ref, requested_at_ns as i64,],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        Ok(())
    }

    /// Story 9.2 (P29) — is this principal under a durable legal hold?
    pub fn is_under_legal_hold(&self, principal_id: &str) -> Result<bool, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let exists: i64 = inner
            .conn
            .query_row(
                "SELECT COUNT(*) FROM legal_holds WHERE principal_id = ?1",
                rusqlite::params![principal_id],
                |row| row.get(0),
            )
            .map_err(AuditError::SqliteRead)?;
        Ok(exists > 0)
    }

    /// Story 9.2 (P29) — release a legal hold so the principal may be erased
    /// again. Returns whether a hold was actually removed.
    pub fn release_legal_hold(&self, principal_id: &str) -> Result<bool, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let removed = inner
            .conn
            .execute(
                "DELETE FROM legal_holds WHERE principal_id = ?1",
                rusqlite::params![principal_id],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        Ok(removed > 0)
    }

    /// Story 9.2 (P7) — journal a kernel `TaskComplete` frame and return its
    /// frame_id, both under one lock acquisition so a concurrent insert cannot
    /// steal `last_frame_id`.  Panics on write failure per the I2 binding,
    /// exactly like `insert_frame_event`.  Used by the forget cascade where the
    /// receipt must name the frame that was just written.
    pub fn insert_kernel_event_returning_id(
        &self,
        spirit_pid: u32,
        intent: &str,
        payload: &[u8],
    ) -> [u8; 16] {
        let redacted = self.redaction.redact(payload);
        let mut inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let frame_id = Self::next_frame_id(&mut inner);
        let timestamp_ns = wall_clock_now_ns();
        inner.last_frame_id = frame_id;
        let result = inner.conn.execute(
            "INSERT INTO transparency_log
                (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce, capability_token,
                 kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                &frame_id[..],
                timestamp_ns as i64,
                spirit_pid as i64,
                "",
                "",
                inner.boot_nonce as i64,
                None::<&[u8]>,
                FrameKind::TaskComplete as i64,
                intent,
                &redacted[..],
                FrameOrigin::Kernel as i64,
            ],
        );
        match result {
            Ok(_) => frame_id,
            Err(e) => {
                panic!(
                    "MAOS I2 binding (Story 9.2 P7): transparency-log write failed for \
                     '{intent}': {e}"
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
    pub fn insert_approval_decision(&self, decision: ApprovalDecision) -> Result<(), AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let timestamp_ns = wall_clock_now_ns();
        inner
            .conn
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
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let mut sql = String::from(
            "SELECT frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce,
                    capability_token, kind, intent, correlation_id, payload_redacted, origin
             FROM transparency_log",
        );
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(pid) = filter.spirit_pid {
            where_clauses.push("spirit_pid = ?".to_string());
            params.push(Box::new(pid as i64));
        }
        if let Some(fid) = filter.frame_id {
            where_clauses.push("frame_id = ?".to_string());
            params.push(Box::new(fid.to_vec()));
        }
        if let Some(kind) = filter.kind {
            where_clauses.push("kind = ?".to_string());
            params.push(Box::new(kind as i64));
        }
        if let Some(correlation_id) = filter.correlation_id {
            where_clauses.push("correlation_id = ?".to_string());
            params.push(Box::new(correlation_id));
        }
        if let Some(since) = filter.since_ns {
            where_clauses.push("timestamp_ns >= ?".to_string());
            params.push(Box::new(since as i64));
        }
        if let Some(until) = filter.until_ns {
            where_clauses.push("timestamp_ns <= ?".to_string());
            params.push(Box::new(until as i64));
        }
        if let (Some(cursor_ts), Some(cursor_fid)) =
            (filter.cursor_timestamp_ns, filter.cursor_frame_id)
        {
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
                let cap_blob: Option<Vec<u8>> = row.get(6)?;
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
                    from_spirit_id: row.get(3)?,
                    to_spirit_id: row.get(4)?,
                    boot_nonce: row.get::<_, i64>(5)? as u64,
                    capability_token: cap_token,
                    kind: FrameKind::from_i64(row.get::<_, i64>(7)?).unwrap_or_else(|| {
                        let disc = row.get::<_, i64>(7).unwrap_or(-1);
                        eprintln!(
                            "TL query: unrecognized FrameKind discriminant ({disc}); \
                                 mapping to TaskAssign as best-effort fallback \
                                 (schema-migration or cross-version log inspection)"
                        );
                        FrameKind::TaskAssign
                    }),
                    intent: row.get(8)?,
                    correlation_id: row.get(9)?,
                    payload_redacted: row.get(10)?,
                    origin: match row.get::<_, i64>(11)? {
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
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let mut stmt = inner
            .conn
            .prepare(
                "SELECT frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce,
                        capability_token, kind, intent, correlation_id, payload_redacted, origin
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
                let cap_blob: Option<Vec<u8>> = row.get(6)?;
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
                    from_spirit_id: row.get(3)?,
                    to_spirit_id: row.get(4)?,
                    boot_nonce: row.get::<_, i64>(5)? as u64,
                    capability_token: cap_token,
                    kind: FrameKind::from_i64(row.get::<_, i64>(7)?).unwrap_or_else(|| {
                        let disc = row.get::<_, i64>(7).unwrap_or(-1);
                        eprintln!(
                            "TL query: unrecognized FrameKind discriminant ({disc}); \
                                 mapping to TaskAssign as best-effort fallback \
                                 (schema-migration or cross-version log inspection)"
                        );
                        FrameKind::TaskAssign
                    }),
                    intent: row.get(8)?,
                    correlation_id: row.get(9)?,
                    payload_redacted: row.get(10)?,
                    origin: match row.get::<_, i64>(11)? {
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
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");

        // Story 9.7 #5 — order by the autoincrement `decision_id` (true
        // insertion order, monotonic) instead of `timestamp_ns`, which is a
        // non-monotonic wall clock (NTP/VM-suspend step-backward). Using
        // `timestamp_ns` for LWW "latest" elects the wrong winner on a backward
        // clock step; `decision_id ASC` is deterministic and monotonic, and
        // reconcile picks the last row per target from this ordering.
        let sql = "SELECT actor, target, capability, intent, decision, reasoning \
                   FROM approval_decision_log \
                   ORDER BY decision_id ASC";

        let mut stmt = inner.conn.prepare(sql).map_err(AuditError::SqliteRead)?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ApprovalDecision {
                    actor: row.get(0)?,
                    target: row.get(1)?,
                    capability: row.get(2)?,
                    intent: row.get(3)?,
                    decision: row.get::<_, i64>(4)? != 0,
                    reasoning: row.get(5)?,
                })
            })
            .map_err(AuditError::SqliteRead)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AuditError::SqliteRead)
    }

    /// Story 6.1 — mark a frame as retracted in the companion table.
    ///
    /// Returns `true` if this was the first retraction, `false` if already
    /// retracted (idempotent).
    pub fn mark_retracted(
        &self,
        original_frame_id: [u8; 16],
        retract_frame_id: [u8; 16],
    ) -> Result<bool, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let timestamp_ns = wall_clock_now_ns();
        let rows = inner
            .conn
            .execute(
                "INSERT OR IGNORE INTO transparency_log_retractions
                 (original_frame_id, retract_frame_id, retracted_at_ns)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    &original_frame_id[..],
                    &retract_frame_id[..],
                    timestamp_ns as i64,
                ],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        Ok(rows > 0)
    }

    /// Story 6.1 — atomically check-and-mark retraction.
    ///
    /// Holds the inner lock across both operations to prevent TOCTOU races.
    /// Returns `Ok(retract_frame_id)` if already retracted, `Ok(None)` if
    /// not yet retracted (caller should proceed with retraction).
    pub fn check_and_mark_retracted(
        &self,
        original_frame_id: [u8; 16],
        retract_frame_id: [u8; 16],
    ) -> Result<Option<[u8; 16]>, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        // Check if already retracted
        let row: Option<Vec<u8>> = inner
            .conn
            .query_row(
                "SELECT retract_frame_id FROM transparency_log_retractions
                 WHERE original_frame_id = ?1 LIMIT 1",
                rusqlite::params![&original_frame_id[..]],
                |row| row.get(0),
            )
            .optional()
            .map_err(AuditError::SqliteRead)?;
        if let Some(blob) = row {
            let mut arr = [0u8; 16];
            if blob.len() == 16 {
                arr.copy_from_slice(&blob);
            }
            return Ok(Some(arr));
        }
        // Not yet retracted — mark it atomically
        let timestamp_ns = wall_clock_now_ns();
        inner
            .conn
            .execute(
                "INSERT OR IGNORE INTO transparency_log_retractions
                 (original_frame_id, retract_frame_id, retracted_at_ns)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    &original_frame_id[..],
                    &retract_frame_id[..],
                    timestamp_ns as i64,
                ],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        Ok(None)
    }

    /// Story 6.1 — check whether a frame has been retracted.
    ///
    /// Returns `Some(retract_frame_id)` if retracted, `None` otherwise.
    pub fn is_retracted(
        &self,
        original_frame_id: [u8; 16],
    ) -> Result<Option<[u8; 16]>, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let row: Option<Vec<u8>> = inner
            .conn
            .query_row(
                "SELECT retract_frame_id FROM transparency_log_retractions
                 WHERE original_frame_id = ?1 LIMIT 1",
                rusqlite::params![&original_frame_id[..]],
                |row| row.get(0),
            )
            .optional()
            .map_err(AuditError::SqliteRead)?;
        Ok(row.map(|blob| {
            let mut arr = [0u8; 16];
            if blob.len() == 16 {
                arr.copy_from_slice(&blob);
            }
            arr
        }))
    }

    /// Get a reference to the mailbox stub (for testing).
    pub fn mailbox(&self) -> &MailboxStub {
        &self.mailbox
    }

    /// Story 9.3b (R10) — append a schema-lifecycle registry entry AND emit
    /// the corresponding governance frame atomically.
    ///
    /// HARD-REJECTS any entry lacking a `ratified_by` ADR reference.
    pub fn register_schema_lifecycle(
        &self,
        entry: &maos_domain::governance::SchemaRegistryEntry,
    ) -> Result<LogBeforeDeliver<()>, AuditError> {
        if entry.ratified_by.is_empty() {
            return Err(AuditError::SqliteWriteFatal(
                rusqlite::Error::InvalidParameterName("ratified_by must not be empty".into()),
            ));
        }
        let mut inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        // Atomic: registry append + frame emission in one SQLite transaction.
        // Manual BEGIN/COMMIT to avoid borrow conflicts with Transaction<'_>.
        inner
            .conn
            .execute_batch("BEGIN;")
            .map_err(AuditError::SqliteWriteFatal)?;
        let commit_or_rollback =
            |conn: &Connection, result: Result<(), AuditError>| -> Result<(), AuditError> {
                match result {
                    Ok(()) => conn
                        .execute_batch("COMMIT;")
                        .map_err(AuditError::SqliteWriteFatal),
                    Err(e) => {
                        let _ = conn.execute_batch("ROLLBACK;");
                        Err(e)
                    }
                }
            };
        let result = (|| -> Result<(), AuditError> {
            inner.conn.execute(
                "INSERT INTO schema_lifecycle_registry
                    (schema_id, version, effective_at_ns, supersedes_hash, ratified_by, recorded_at_ns, schema_content_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    entry.schema_id,
                    entry.version as i64,
                    entry.effective_at_ns as i64,
                    entry.supersedes_hash.as_deref(),
                    entry.ratified_by,
                    entry.recorded_at_ns as i64,
                    entry.schema_content_hash,
                ],
            ).map_err(|e| {
                panic!("MAOS kernel panic — schema_lifecycle_registry write failed: {e}. I2.");
            }).unwrap();
            // Build governance frame payload
            let payload = maos_domain::governance::GovernanceEventPayload {
                recorded_at_ns: entry.recorded_at_ns,
                effective_at_ns: entry.effective_at_ns,
                event: maos_domain::governance::GovernanceEventKind::SchemaLifecycle(
                    maos_domain::governance::SchemaLifecyclePayload {
                        schema_id: entry.schema_id.clone(),
                        schema_content_hash: entry.schema_content_hash.clone(),
                        supersedes: entry.supersedes_hash.clone(),
                        version: entry.version,
                        ratified_by: entry.ratified_by.clone(),
                    },
                ),
            };
            let payload_bytes = serde_json::to_vec(&payload)
                .map_err(|e| AuditError::Serialization(e.to_string()))?;
            let redacted = self.redaction.redact(&payload_bytes);
            let frame_id_val = Self::next_frame_id(&mut inner);
            let timestamp_ns = wall_clock_now_ns();
            inner.last_frame_id = frame_id_val;
            inner.conn.execute(
                "INSERT INTO transparency_log
                    (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce, capability_token,
                     kind, intent, payload_redacted, origin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    &frame_id_val[..],
                    timestamp_ns as i64,
                    0i64,
                    "",
                    "",
                    inner.boot_nonce as i64,
                    Option::<&[u8]>::None,
                    FrameKind::GovernanceEvent as i64,
                    "governance:schema-lifecycle",
                    &redacted[..],
                    FrameOrigin::Kernel as i64,
                ],
            ).map_err(|e| {
                panic!("MAOS kernel panic — Transparency Log write failed: {e}. I2.");
            }).unwrap();
            Ok(())
        })();
        commit_or_rollback(&inner.conn, result)?;
        Ok(LogBeforeDeliver::new(()))
    }

    /// Story 9.3b (R10) — query the schema-lifecycle registry for a specific
    /// schema_id. Returns entries ordered by version ascending.
    pub fn query_schema_registry(
        &self,
        schema_id: &str,
    ) -> Result<Vec<maos_domain::governance::SchemaRegistryEntry>, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let mut stmt = inner
            .conn
            .prepare(
                "SELECT schema_id, version, effective_at_ns, supersedes_hash, ratified_by, recorded_at_ns, schema_content_hash
             FROM schema_lifecycle_registry WHERE schema_id = ?1 ORDER BY version ASC",
            )
            .map_err(AuditError::SqliteRead)?;
        let rows = stmt
            .query_map(rusqlite::params![schema_id], |row| {
                Ok(maos_domain::governance::SchemaRegistryEntry {
                    schema_id: row.get(0)?,
                    version: row.get::<_, i64>(1)? as u32,
                    effective_at_ns: row.get::<_, i64>(2)? as u64,
                    supersedes_hash: row.get(3)?,
                    ratified_by: row.get(4)?,
                    recorded_at_ns: row.get::<_, i64>(5)? as u64,
                    schema_content_hash: row.get(6)?,
                })
            })
            .map_err(AuditError::SqliteRead)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(AuditError::SqliteRead)?);
        }
        Ok(out)
    }

    /// Story 9.3b — query the schema version currently in force for a
    /// given schema_id (the latest by version number).
    pub fn current_schema_version(
        &self,
        schema_id: &str,
    ) -> Result<Option<maos_domain::governance::SchemaRegistryEntry>, AuditError> {
        let inner = self
            .inner
            .lock()
            .expect("TransparencyLogAdapter inner poisoned");
        let mut stmt = inner
            .conn
            .prepare(
                "SELECT schema_id, version, effective_at_ns, supersedes_hash, ratified_by, recorded_at_ns, schema_content_hash
             FROM schema_lifecycle_registry WHERE schema_id = ?1 ORDER BY version DESC LIMIT 1",
            )
            .map_err(AuditError::SqliteRead)?;
        let mut rows = stmt
            .query_map(rusqlite::params![schema_id], |row| {
                Ok(maos_domain::governance::SchemaRegistryEntry {
                    schema_id: row.get(0)?,
                    version: row.get::<_, i64>(1)? as u32,
                    effective_at_ns: row.get::<_, i64>(2)? as u64,
                    supersedes_hash: row.get(3)?,
                    ratified_by: row.get(4)?,
                    recorded_at_ns: row.get::<_, i64>(5)? as u64,
                    schema_content_hash: row.get(6)?,
                })
            })
            .map_err(AuditError::SqliteRead)?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(AuditError::SqliteRead(e)),
            None => Ok(None),
        }
    }
}

/// One side of a correlation query, retaining the physical team provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTransparencyLogEntry {
    pub team_id: maos_domain::team::TeamId,
    pub entry: TransparencyLogEntry,
}

/// Reconcile a correlation token across explicitly supplied team artifacts.
///
/// A single adapter can only return its local half. Callers must enumerate
/// every team path participating in the action; this function preserves each
/// row's physical-team provenance and returns one deterministic timeline.
pub fn reconcile_correlated_frames(
    sources: &[(&maos_domain::team::TeamId, &TransparencyLogAdapter)],
    correlation_id: &str,
) -> Result<Vec<TeamTransparencyLogEntry>, AuditError> {
    let mut reconciled = Vec::new();
    for (team_id, log) in sources {
        let entries = log.query_frames(FrameFilter {
            correlation_id: Some(correlation_id.to_owned()),
            ..Default::default()
        })?;
        reconciled.extend(entries.into_iter().map(|entry| TeamTransparencyLogEntry {
            team_id: (*team_id).clone(),
            entry,
        }));
    }
    reconciled.sort_by(|left, right| {
        (
            left.entry.timestamp_ns,
            left.entry.frame_id,
            left.team_id.as_str(),
        )
            .cmp(&(
                right.entry.timestamp_ns,
                right.entry.frame_id,
                right.team_id.as_str(),
            ))
    });
    Ok(reconciled)
}

/// Format a frame_id as colon-separated hex pairs.
fn format_frame_id_hex(frame_id: &[u8; 16]) -> String {
    frame_id
        .chunks(2)
        .map(|chunk| format!("{:02x}{:02x}", chunk[0], chunk[1]))
        .collect::<Vec<_>>()
        .join(":")
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
        let payload_bytes = serde_json::to_vec(&frame.payload).map_err(|e| {
            maos_domain::iac_bus_types::IacBusError::SerializationFailed(e.to_string())
        })?;
        Ok(self.enqueue_frame(&payload_bytes, frame.auto_marker))
    }

    fn register_spirit(
        &self,
        _spirit_id: &maos_spirit_abi::identity::SpiritId,
    ) -> Result<(), maos_domain::iac_bus_types::IacBusError> {
        Ok(())
    }

    async fn retract(
        &self,
        _original_frame_id: [u8; 16],
        _reason: String,
        _retracting_spirit: &maos_spirit_abi::identity::SpiritId,
    ) -> Result<maos_domain::iac_bus_types::RetractOutcome, maos_domain::iac_bus_types::IacBusError>
    {
        // Stub implementation — the real retract lives in IacBusAdapter.
        // This stub satisfies the trait for TransparencyLogAdapter which
        // is only used in test contexts as a standalone IacBusPort impl.
        Ok(maos_domain::iac_bus_types::RetractOutcome::OriginalNotFound)
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

// Story 6.5 — HaltJournal trait impl moved from maos-kernel-core to maos-iac
// per orphan rules (TransparencyLogAdapter is now defined in maos-iac).
impl maos_domain::halt::HaltJournal for TransparencyLogAdapter {
    fn journal_halt_resolution(
        &self,
        actor: &str,
        spirit_id: &str,
        halt_id: &maos_domain::halt::HaltId,
        resolution: &maos_domain::halt::Resolution,
    ) -> Result<(), maos_domain::halt::HaltJournalError> {
        let reasoning = match resolution {
            maos_domain::halt::Resolution::ProvidedContext { text } => Some(format!(
                "halt={}: provided_context: {text}",
                halt_id.as_str()
            )),
            maos_domain::halt::Resolution::AcceptedHalt => {
                Some(format!("halt={}: accepted_halt", halt_id.as_str()))
            }
            maos_domain::halt::Resolution::AuthorizedOverride {
                operator_policy_ref,
            } => Some(format!(
                "halt={}: authorized_override: operator_policy_ref={operator_policy_ref}",
                halt_id.as_str()
            )),
        };
        self.insert_approval_decision(ApprovalDecision {
            actor: actor.into(),
            target: spirit_id.into(),
            capability: "halt.resolve".into(),
            intent: resolution.kind_label().into(),
            decision: true,
            reasoning,
        })
        .map_err(|e| maos_domain::halt::HaltJournalError::WriteFailed(e.to_string()))
    }
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
    fn cross_team_correlation_requires_both_physical_logs() {
        let security = maos_domain::team::TeamId::new("security").unwrap();
        let support = maos_domain::team::TeamId::new("support").unwrap();
        let security_log = TransparencyLogAdapter::open_in_memory(1);
        let support_log = TransparencyLogAdapter::open_in_memory(2);
        let correlation_id = "cross-team-action-01";

        for (log, pid, intent) in [
            (&security_log, 7, "cross-team-send"),
            (&support_log, 8, "cross-team-receive"),
        ] {
            let _ = log.insert_frame_event_with_correlation(
                FrameKind::CapabilityInvocation,
                pid,
                None,
                correlation_id,
                intent,
                b"redacted",
                FrameOrigin::Kernel,
            );
        }

        let one_path = security_log
            .query_frames(FrameFilter {
                correlation_id: Some(correlation_id.to_owned()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            one_path.len(),
            1,
            "a one-path reader cannot reconcile both team records"
        );

        let reconciled = reconcile_correlated_frames(
            &[(&security, &security_log), (&support, &support_log)],
            correlation_id,
        )
        .unwrap();
        assert_eq!(reconciled.len(), 2);
        assert_eq!(
            reconciled[0].entry.correlation_id.as_deref(),
            Some(correlation_id)
        );
        assert_eq!(
            reconciled[1].entry.correlation_id.as_deref(),
            Some(correlation_id)
        );
        assert_ne!(reconciled[0].team_id, reconciled[1].team_id);
    }

    #[test]
    fn correlation_column_migrates_an_existing_transparency_log() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("legacy.sqlite");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                from_spirit_id TEXT NOT NULL DEFAULT '',
                to_spirit_id TEXT NOT NULL DEFAULT '',
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

        let log = TransparencyLogAdapter::open(&path, 3).unwrap();
        let _ = log.insert_frame_event_with_correlation(
            FrameKind::CapabilityInvocation,
            9,
            None,
            "migrated-correlation",
            "migrated-action",
            b"redacted",
            FrameOrigin::Kernel,
        );
        let entries = log
            .query_frames(FrameFilter {
                correlation_id: Some("migrated-correlation".to_owned()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].correlation_id.as_deref(),
            Some("migrated-correlation")
        );
    }

    #[test]
    fn team_shard_keeps_legal_holds_in_global_database() {
        let dir = tempfile::TempDir::new().unwrap();
        let team_path = dir.path().join("teams/security/transparency.sqlite");
        let global_path = dir.path().join("transparency.sqlite");
        std::fs::create_dir_all(team_path.parent().unwrap()).unwrap();

        let log = TransparencyLogAdapter::open(&team_path, 4).unwrap();
        log.attach_global_legal_holds(&global_path).unwrap();
        log.place_legal_hold("principal-1", "legal-hold", Some("case-1"), 10)
            .unwrap();

        let team = rusqlite::Connection::open(&team_path).unwrap();
        let team_tables: i64 = team
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'legal_holds'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            team_tables, 0,
            "legal_holds must not move into the team shard"
        );

        let global = rusqlite::Connection::open(&global_path).unwrap();
        let global_holds: i64 = global
            .query_row("SELECT count(*) FROM legal_holds", [], |row| row.get(0))
            .unwrap();
        assert_eq!(global_holds, 1);
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
        let entries = log
            .query_frames(FrameFilter {
                spirit_pid: Some(7),
                ..Default::default()
            })
            .unwrap();
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
        let filtered = log
            .query_frames(FrameFilter {
                since_ns: Some(since),
                ..Default::default()
            })
            .unwrap();
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
        assert_eq!(
            entries.len(),
            40,
            "expected 40 rows from 4 threads × 10 inserts"
        );
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
        })
        .unwrap();

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
        })
        .unwrap();

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
            vec![
                "approval_decision_log",
                "legal_holds",
                // Story 9.3b (commit 3041fec) added the schema-lifecycle registry
                // table at open; this exhaustive list was left stale until the
                // Story 9.4b AC-6 regression caught it.
                "schema_lifecycle_registry",
                "transparency_log",
                "transparency_log_retractions"
            ],
            "expected exactly five tables"
        );

        // 2. No foreign keys on transparency_log
        let mut stmt = conn
            .prepare("PRAGMA foreign_key_list(transparency_log)")
            .unwrap();
        let fk_tlog: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(fk_tlog.is_empty(), "transparency_log has foreign keys");

        // 3. No foreign keys on approval_decision_log
        let mut stmt = conn
            .prepare("PRAGMA foreign_key_list(approval_decision_log)")
            .unwrap();
        let fk_adl: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(fk_adl.is_empty(), "approval_decision_log has foreign keys");

        // 4. Each table has exactly 1 row
        let tlog_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transparency_log", [], |row| {
                row.get(0)
            })
            .unwrap();
        let adl_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM approval_decision_log", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(tlog_count, 1);
        assert_eq!(adl_count, 1);

        // 5. Deleting from one table does not affect the other (test-only DELETE)
        conn.execute("DELETE FROM transparency_log", []).unwrap();
        let adl_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM approval_decision_log", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            adl_after, 1,
            "approval row affected by transparency_log DELETE"
        );
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
            let old_conn =
                std::mem::replace(&mut inner.conn, Connection::open_in_memory().unwrap());
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

    // ---- Story 5.3 — FrameKind variant tests ----

    #[test]
    fn frame_kind_task_stalled_from_i64() {
        assert_eq!(FrameKind::from_i64(15), Some(FrameKind::TaskStalled));
    }

    #[test]
    fn frame_kind_silent_failure_suspect_from_i64() {
        assert_eq!(
            FrameKind::from_i64(16),
            Some(FrameKind::SilentFailureSuspect)
        );
    }

    #[test]
    fn frame_kind_non_exhaustive_match() {
        let kind = FrameKind::TaskStalled;
        let _name = match kind {
            FrameKind::TaskStalled => "stalled",
            FrameKind::SilentFailureSuspect => "silent_failure",
            // All other variants — the #[non_exhaustive] on the enum means
            // downstream crates need a wildcard, but within the defining crate
            // we can match exhaustively.
            _ => "other",
        };
    }

    #[test]
    fn frame_kind_spirit_revoked_from_i64() {
        assert_eq!(FrameKind::from_i64(17), Some(FrameKind::SpiritRevoked));
    }

    #[test]
    fn frame_kind_spirit_revoked_discriminant() {
        assert_eq!(FrameKind::SpiritRevoked as i64, 17);
    }
}
