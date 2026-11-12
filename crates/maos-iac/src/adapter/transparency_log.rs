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

use maos_domain::frame::{ConsentEnvelope, ConsentRecall};
use maos_domain::invariants::i2::LogBeforeDeliver;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i4::ApprovalDecision;
use maos_domain::memory::{principal_id_occurs, RedistillOutcome, RedistillRequest};
use maos_domain::ports::mcp::McpError;
use maos_domain::ports::IacBusPort;
use rusqlite::{Connection, OptionalExtension};

use super::mailbox_stub::MailboxStub;
use super::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};

mod erasure_sweep;
pub use erasure_sweep::{ErasureSweep, SweepCompletion};
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
    /// Story 18-5 — ADR-016: self-reported context crossed the pressure threshold.
    ContextPressure = 32,
    /// Story 18-5 — ADR-016: self-reported context crossed the limit threshold.
    ContextLimit = 33,
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
            32 => Some(Self::ContextPressure),
            33 => Some(Self::ContextLimit),
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
    /// SQLite insertion order. Stable under wall-clock corrections.
    pub insertion_id: i64,
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

/// Cascade metadata only; candidate bodies are streamed and never retained.
pub struct ForgetFrame {
    pub frame_id: [u8; 16],
    pub timestamp_ns: i64,
    pub kind: i64,
    pub spirit_pid: u32,
    pub boot_nonce: u64,
    pub source_log_ref: Vec<String>,
}

struct ErasureTarget<'a> {
    frame_id: &'a [u8; 16],
    kind: i64,
    id: &'a str,
    principals: &'a [String],
    spirit: Option<&'a str>,
    reason: &'a str,
    request: Option<&'a RedistillRequest>,
}
enum ErasureWrite {
    Retry,
    Held,
    Redacted,
}

/// Payload-free row metadata. The raw kind is deliberately not decoded through
/// the audit reader's compatibility fallback. `payload_len` is SQLite's stored
/// blob length, so consumers can reject oversized fetches before reading it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparencyLogMetadata {
    pub frame_id: [u8; 16],
    pub timestamp_ns: u64,
    pub spirit_pid: u32,
    pub boot_nonce: u64,
    pub raw_kind: i64,
    pub intent: String,
    pub payload_len: u64,
}

/// Filter for `query_frames` and `query_approvals`. v0.1-β supports the
/// minimum needed by Story 1b.5b's `maosctl audit query --spirit <name>`;
/// extensions (subject-access, posture-delta) ship in Story 9.1.
#[derive(Debug, Clone, Default)]
pub struct FrameFilter {
    pub spirit_pid: Option<u32>,
    pub boot_nonce: Option<u64>,
    /// Restrict the raw SQLite kind to Spirit-disclosable bus/MCP frames.
    pub disclosable_kinds_only: bool,
    pub intent: Option<String>,
    pub kind: Option<FrameKind>,
    pub correlation_id: Option<String>,
    pub since_ns: Option<u64>,
    pub until_ns: Option<u64>,
    pub limit: Option<usize>,
    /// Return newest rows first. Cursors remain lower-bound cursors; callers
    /// combining them with descending order own that pagination contract.
    pub descending: bool,
    /// Order by SQLite insertion order instead of wall-clock timestamp.
    pub order_by_insertion: bool,
    /// Story 6.1 — filter by exact frame_id (for retract authority lookup).
    pub frame_id: Option<[u8; 16]>,
    /// Keyset-pagination cursor — exclusive lower bound on `(timestamp_ns, frame_id)`.
    /// Both components are required unless `order_by_insertion` is set.
    pub cursor_timestamp_ns: Option<u64>,
    /// In insertion order, resolve this frame's rowid as the exclusive anchor.
    pub cursor_frame_id: Option<[u8; 16]>,
    /// Exclusive insertion-order cursor (`rowid > cursor_insertion_id`).
    pub cursor_insertion_id: Option<i64>,
    /// Story 17-8 AC2.5 — participant scope: rows the caller sent or was
    /// recorded as receiving, each received row still under its consent at
    /// `consent_now_ns`. Reads the side tables, so never set on a foreign
    /// (cross-wall) artifact.
    pub participant: Option<ParticipantScope>,
}

/// Story 17-8 — a participant caller and the consent clock reading it is judged at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticipantScope {
    pub pid: u32,
    pub consent_now_ns: u64,
}

/// Story 17-8 AC2.6 — how one stored frame discloses to a participant caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantAccess {
    /// Foreign boot, non-disclosable kind, or neither sender nor recorded recipient.
    OutOfScope { owner_pid: u32 },
    /// A recorded recipient whose granter's consent withholds the payload
    /// (expired, ruptured since delivery, `Withheld`, `HeaderOnly` or unknown).
    Withheld,
    /// The payload may be loaded; `payload_len` is its stored length.
    Payload { payload_len: u64 },
}

/// Story 17-8 CR-1 — one recorded local recipient of a routed frame: an
/// admitted incarnation resolved through the [`RecipientResolver`], never a
/// pid taken from the frame, the wire or a guest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameRecipient {
    pub pid: u32,
    pub spirit_id: String,
    pub role: Option<maos_spirit_abi::identity::SpiritRole>,
}

/// Story 17-8 AC2.3 — resolves local addressees to admitted incarnations. The
/// composition root installs exactly one, over `pid_by_spirit_id`.
pub trait RecipientResolver: Send + Sync + std::fmt::Debug {
    /// Resolve every id under ONE read of the admitted-incarnation map.
    fn resolve(&self, spirit_ids: &[&str]) -> Vec<Option<u32>>;
}

/// The admitted-incarnation map itself (`pid_by_spirit_id`). A poisoned map
/// resolves nobody: a received frame then stays undisclosable (fail-closed).
impl RecipientResolver for std::sync::RwLock<std::collections::BTreeMap<String, u32>> {
    fn resolve(&self, spirit_ids: &[&str]) -> Vec<Option<u32>> {
        let Ok(map) = self.read() else {
            return vec![None; spirit_ids.len()];
        };
        spirit_ids.iter().map(|id| map.get(*id).copied()).collect()
    }
}

/// Story 17-8 AC3.1 — the one order-preserving, lossless `u64 → i64` key for
/// consent timestamps and the bound `now` (`u64::MAX` and other values above
/// `i64::MAX` are real: the fail-closed clock, saturating TTLs). Never clamp.
pub fn consent_order_key(value: u64) -> i64 {
    (value ^ (1 << 63)) as i64
}

/// Story 17-3d R17-159 — the raw kinds a Spirit may recall (bus frames and MCP
/// invocations). The one list shared by the Rust check and the SQL predicate.
pub const SPIRIT_DISCLOSABLE_RAW_KINDS: [i64; 8] = [0, 1, 2, 3, 4, 5, 6, 18];

fn disclosable_kinds_sql(column: &str) -> String {
    let kinds = SPIRIT_DISCLOSABLE_RAW_KINDS.map(|kind| kind.to_string());
    format!("{column} IN ({})", kinds.join(","))
}

/// Story 17-8 AC2.5 / AC3.3 — `caller` is the one-row `(pid, now_key)` CTE.
/// A received row needs a recipient record naming the caller; one with a
/// consent record is withheld when its policy is not Full/HeaderOnly (unknown
/// integers included), when expired at `now`, or when a rupture of the same
/// granter and exact intent class naming the caller was journaled after it in
/// this boot (insertion order, never wall time; `IS` is NULL-safe).
///
/// The recipient test is written as `frame_id IN (subquery)` rather than a
/// correlated `EXISTS` so the planner can serve `sender OR recipient` as a
/// `MULTI-INDEX OR` over `idx_tlog_boot_pid_ts` and `idx_tlog_recipient_pid`
/// instead of scanning every row of the boot (code review 2026-10-21). Every
/// `OR` term must stay indexable or the planner silently falls back to a scan.
const RECIPIENT_PREDICATE: &str = "t.frame_id IN (SELECT r.frame_id \
     FROM transparency_log_recipients r WHERE r.recipient_pid = caller.pid)";
const CONSENT_PREDICATE: &str = "NOT EXISTS (SELECT 1 FROM transparency_log_consent c \
     WHERE c.frame_id = t.frame_id AND (c.recall_policy NOT IN (0, 1) \
     OR c.valid_until_key < caller.now_key \
     OR EXISTS (SELECT 1 FROM transparency_log_consent_ruptures x \
       JOIN transparency_log tr ON tr.frame_id = x.rupture_frame_id \
       WHERE x.recipient_pid = caller.pid AND tr.boot_nonce = t.boot_nonce \
       AND tr.rowid > t.rowid AND x.granter_spirit_id = c.granter_spirit_id \
       AND x.granter_host_id IS c.granter_host_id AND x.intent_class IS c.intent_class)))";
const HEADER_ONLY_PREDICATE: &str = "EXISTS (SELECT 1 FROM transparency_log_consent h \
     WHERE h.frame_id = t.frame_id AND h.recall_policy = 1)";

fn participant_predicate() -> String {
    format!("(t.spirit_pid = caller.pid OR ({RECIPIENT_PREDICATE} AND {CONSENT_PREDICATE}))")
}

const FRAME_METADATA_PROJECTION: &str =
    "frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, length(payload_redacted)";

/// Story 17-8 — rows that commit in their base row's transaction.
enum SideRows<'a> {
    /// AC2.2 — a routed frame's recorded recipients and its consent envelope.
    Routed {
        recipients: &'a [FrameRecipient],
        consent: Option<&'a ConsentEnvelope>,
    },
    /// AC3.5 — a rupture of `envelope`'s grant naming each rejected recipient.
    Rupture {
        envelope: &'a ConsentEnvelope,
        recipient_pids: &'a [u32],
    },
}

impl SideRows<'_> {
    fn write(self, conn: &Connection, frame_id: &[u8; 16]) -> rusqlite::Result<()> {
        let role =
            |role: Option<maos_spirit_abi::identity::SpiritRole>| role.map(|r| format!("{r:?}"));
        match self {
            Self::Routed {
                recipients,
                consent,
            } => {
                for recipient in recipients {
                    conn.execute(
                        "INSERT INTO transparency_log_recipients
                            (frame_id, recipient_pid, recipient_spirit_id, recipient_role)
                         VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![
                            &frame_id[..],
                            recipient.pid as i64,
                            recipient.spirit_id,
                            role(recipient.role),
                        ],
                    )?;
                }
                let Some(envelope) = consent else {
                    return Ok(());
                };
                let policy: i64 = match envelope.recall {
                    ConsentRecall::Full => 0,
                    ConsentRecall::HeaderOnly => 1,
                    ConsentRecall::Withheld => 2,
                };
                conn.execute(
                    "INSERT INTO transparency_log_consent
                        (frame_id, consent_id, granter_spirit_id, granter_host_id, granter_role,
                         granted_at_key, intent_class, valid_until_key, recall_policy)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        &frame_id[..],
                        &envelope.consent_id[..],
                        envelope.granter.spirit_id.as_str(),
                        envelope.granter.host_id.as_ref().map(|host| host.as_str()),
                        role(envelope.granter.role),
                        consent_order_key(envelope.timestamp_ns),
                        envelope.intent_class.as_ref().map(|intent| intent.as_str()),
                        envelope.valid_until_ns.map(consent_order_key),
                        policy,
                    ],
                )?;
                Ok(())
            }
            Self::Rupture {
                envelope,
                recipient_pids,
            } => {
                for pid in recipient_pids {
                    conn.execute(
                        "INSERT OR IGNORE INTO transparency_log_consent_ruptures
                            (rupture_frame_id, recipient_pid, granter_spirit_id, granter_host_id,
                             intent_class)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![
                            &frame_id[..],
                            *pid as i64,
                            envelope.granter.spirit_id.as_str(),
                            envelope.granter.host_id.as_ref().map(|host| host.as_str()),
                            envelope.intent_class.as_ref().map(|intent| intent.as_str()),
                        ],
                    )?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PersistedLegalHold {
    pub principal_id: String,
    pub reason: String,
    pub case_ref: Option<String>,
    pub requested_at_ns: u64,
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
    #[error("legal-hold authority is not bound")]
    LegalHoldAuthorityUnbound,
    #[error("erasure pending: the finite frame/hold sweep could not certify completion; partial redactions may have occurred")]
    ErasurePending,
}

/// j1-crosshost-2b AC3.2 — the outcome of one Transparency Log row write.
///
/// The shape mirrors `DigestReplyObservation::{Accepted, Duplicate, Unauthorized}`
/// (`crates/maos-a2a-core/src/cohort.rs:285-296`) — the vocabulary, not its
/// in-memory implementation. **This is not a general dedup store** and no such
/// store was built (AC3.5): the uniqueness constraint already lives at the storage
/// layer, where `frame_id` is `BLOB NOT NULL PRIMARY KEY`, and a second in-memory
/// guard in front of it would be the weaker of the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameRowWrite {
    /// The row was newly inserted.
    Written,
    /// A row with this `frame_id` was already present. The I2 log-before-deliver
    /// guarantee still holds — the frame IS journaled — so the caller decides
    /// idempotency instead of the kernel halting on a peer's replay.
    Duplicate,
}

/// The single layout of a capability token in the `capability_token` column:
/// the token's 16 bytes followed by 16 zero bytes (Story 17-3c; shared by the
/// capability audit writer and IAC bus deliveries).
pub fn capability_token_column(token: &maos_domain::invariants::i1::TokenId) -> [u8; 32] {
    std::array::from_fn(|i| token.0.get(i).copied().unwrap_or(0))
}

const FRAME_ROW_INSERT_SQL: &str = "INSERT INTO transparency_log
                (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce,
                 capability_token, kind, intent, correlation_id, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)";

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
-- Retire L1's full indexes on existing databases as well as fresh installs.
DROP INDEX IF EXISTS idx_tlog_kind_rowid;
DROP INDEX IF EXISTS idx_tlog_intent_rowid;
-- The implicit rowid tail makes the erasure-only predicates seekable without
-- maintaining two additional B-trees for every ordinary routed append.
CREATE INDEX IF NOT EXISTS idx_tlog_distillate_rowid
    ON transparency_log(kind) WHERE kind = 11;
CREATE INDEX IF NOT EXISTS idx_tlog_redistill_request_rowid
    ON transparency_log(intent) WHERE intent = 'distillate.redistill.requested';
-- Opaque TL payloads remain writable. X'' marks invalid JSON, never a JSON id.
CREATE INDEX IF NOT EXISTS idx_tlog_redistill_outcome
    ON transparency_log(CASE WHEN json_valid(CAST(payload_redacted AS TEXT),2)
        THEN json_extract(CAST(payload_redacted AS TEXT),'$.request.redacted_distillate_frame_id')
        ELSE X'' END)
    WHERE intent IN ('distillate.redistilled','distillate.redistill.pending');
CREATE INDEX IF NOT EXISTS idx_tlog_boot_pid_ts
    ON transparency_log(boot_nonce, spirit_pid, timestamp_ns, frame_id);

-- Story 6.1 — retraction companion table.
-- Per AC2: original row is APPEND-ONLY-PRESERVED.
-- The retraction marker lives in this companion table, not as a column
-- on transparency_log, to minimize schema migration blast radius.
CREATE TABLE IF NOT EXISTS transparency_log_retractions (
    original_frame_id   BLOB    NOT NULL PRIMARY KEY,
    retract_frame_id    BLOB    NOT NULL,
    retracted_at_ns     INTEGER NOT NULL
);

-- Story 17-8 — participant recall side tables, written in the routed frame's
-- (or the kind-22 rupture row's) own transaction. `*_key` columns hold
-- `consent_order_key(u64)`; NULL valid_until = open-ended.
CREATE TABLE IF NOT EXISTS transparency_log_recipients (
    frame_id            BLOB    NOT NULL,
    recipient_pid       INTEGER NOT NULL,
    recipient_spirit_id TEXT    NOT NULL,
    recipient_role      TEXT,
    PRIMARY KEY (frame_id, recipient_pid)
);
CREATE INDEX IF NOT EXISTS idx_tlog_recipient_pid
    ON transparency_log_recipients(recipient_pid, frame_id);
CREATE TABLE IF NOT EXISTS transparency_log_consent (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    consent_id          BLOB    NOT NULL,
    granter_spirit_id   TEXT    NOT NULL,
    granter_host_id     TEXT,
    granter_role        TEXT,
    granted_at_key      INTEGER NOT NULL,
    intent_class        TEXT,
    valid_until_key     INTEGER,
    recall_policy       INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS transparency_log_consent_ruptures (
    rupture_frame_id    BLOB    NOT NULL,
    recipient_pid       INTEGER NOT NULL,
    granter_spirit_id   TEXT    NOT NULL,
    granter_host_id     TEXT,
    intent_class        TEXT,
    PRIMARY KEY (rupture_frame_id, recipient_pid)
);
CREATE INDEX IF NOT EXISTS idx_tlog_rupture_recipient
    ON transparency_log_consent_ruptures(recipient_pid, granter_spirit_id);

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

const LEGAL_HOLDS_TABLE_SQL: &str = "\
CREATE TABLE IF NOT EXISTS legal_holds (
    principal_id        TEXT    NOT NULL PRIMARY KEY,
    reason              TEXT    NOT NULL,
    case_ref            TEXT,
    requested_at_ns     INTEGER NOT NULL,
    hold_id             BLOB NOT NULL DEFAULT (randomblob(16))
);";
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
    // A queued log-before-deliver append gets the next turn between scan windows.
    inner: parking_lot::FairMutex<TransparencyLogInner>,
    // Serializes a hold recheck with each body mutation, never an entire scan.
    erasure_holds: Mutex<()>,
    redaction: Box<dyn RedactionPolicy + Send + Sync>,
    mailbox: MailboxStub,
    /// Story 17-8 AC2.3 — set-once; every routed and rupture writer resolves
    /// recipients through it. Absent ⇒ no recipient rows (fail-closed).
    recipient_resolver: std::sync::OnceLock<std::sync::Arc<dyn RecipientResolver>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegalHoldAuthority {
    Main,
    Attached,
}

struct HoldIdentity([u8; 16]);
impl rusqlite::types::FromSql for HoldIdentity {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value
            .as_blob()?
            .try_into()
            .map(Self)
            .map_err(|error| rusqlite::types::FromSqlError::Other(Box::new(error)))
    }
}

struct TransparencyLogInner {
    conn: Connection,
    next_frame_id_counter: u64,
    boot_nonce: u64,
    last_frame_id: [u8; 16],
    legal_hold_authority: Option<LegalHoldAuthority>,
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
        self.inner.lock().last_frame_id
    }

    /// Story 17-8 AC2.3 — install the one recipient resolver (composition root,
    /// over `pid_by_spirit_id`). Set-once: `Err` if one is already installed.
    pub fn install_recipient_resolver(
        &self,
        resolver: std::sync::Arc<dyn RecipientResolver>,
    ) -> Result<(), ()> {
        self.recipient_resolver.set(resolver).map_err(drop)
    }

    /// The installed recipient resolver, if any.
    pub fn recipient_resolver(&self) -> Option<&dyn RecipientResolver> {
        self.recipient_resolver.get().map(|resolver| &**resolver)
    }

    /// Story 17-8 CR-4 — mint one kernel frame id ahead of a write, so every
    /// row an operation produces (base, side rows, markers) names the same id.
    pub fn mint_frame_id(&self) -> [u8; 16] {
        let mut inner = self.inner.lock();
        Self::next_frame_id(&mut inner.next_frame_id_counter)
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

    /// Open an existing Transparency Log for query-only cross-wall disclosure.
    ///
    /// SQLite enforces read-only mode and `NOFOLLOW`; no schema migration,
    /// journal-mode change, or file creation is attempted.
    pub fn open_read_only(path: &Path) -> Result<Self, AuditError> {
        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        Ok(Self::from_read_only_connection(conn))
    }

    /// Wrap an externally-opened **read-only** SQLite connection for query-only
    /// cross-wall disclosure (Story 13.6d P2 — single-connection foreign read).
    ///
    /// The connection MUST already be read-only with `NOFOLLOW` (open it via
    /// `maos_audit::open_tenant_artifact_readonly`); this constructor performs no
    /// schema work and assumes a query-only handle. No counters or nonce are
    /// seeded — query paths never write, so the zeroed frame-id state is inert.
    /// Redaction is the same stateless `CorpusBackedRedactionPolicy` the local
    /// read uses, so a foreign shard is redacted identically.
    pub fn from_read_only_connection(conn: Connection) -> Self {
        Self {
            erasure_holds: Mutex::new(()),
            inner: parking_lot::FairMutex::new(TransparencyLogInner {
                conn,
                next_frame_id_counter: 0,
                boot_nonce: 0,
                last_frame_id: [0; 16],
                legal_hold_authority: None,
            }),
            redaction: Box::new(CorpusBackedRedactionPolicy::new()),
            mailbox: MailboxStub::new(),
            recipient_resolver: Default::default(),
        }
    }

    /// Open a Transparency Log with an explicit Host-global legal-hold
    /// authority. Team shards attach `global_path`; the global artifact binds
    /// its own `main` schema. Callers using [`Self::open`] remain unbound and
    /// legal-hold reads fail closed.
    pub fn open_with_global_legal_holds(
        path: &Path,
        global_path: &Path,
        boot_nonce: u64,
    ) -> Result<Self, AuditError> {
        let adapter = Self::open(path, boot_nonce)?;
        if path == global_path {
            adapter.bind_main_legal_holds()?;
        } else {
            adapter.attach_global_legal_holds(global_path)?;
        }
        Ok(adapter)
    }

    fn bind_main_legal_holds(&self) -> Result<(), AuditError> {
        let mut inner = self.inner.lock();
        inner
            .conn
            .execute_batch(LEGAL_HOLDS_TABLE_SQL)
            .map_err(AuditError::SqliteWriteFatal)?;
        Self::ensure_hold_identity(&mut inner.conn, LegalHoldAuthority::Main)?;
        inner.legal_hold_authority = Some(LegalHoldAuthority::Main);
        Ok(())
    }

    fn ensure_hold_identity(
        conn: &mut Connection,
        authority: LegalHoldAuthority,
    ) -> Result<(), AuditError> {
        let transaction = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(AuditError::SqliteWriteFatal)?;
        let schema = match authority {
            LegalHoldAuthority::Main => "main",
            LegalHoldAuthority::Attached => "maos_host_global",
        };
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('legal_holds',?1) WHERE name='hold_id')",
            [schema], |row| row.get(0),
        ).map_err(AuditError::SqliteRead)?;
        if !exists {
            transaction
                .execute_batch(match authority {
                    LegalHoldAuthority::Main => {
                        "ALTER TABLE main.legal_holds ADD COLUMN hold_id BLOB;"
                    }
                    LegalHoldAuthority::Attached => {
                        "ALTER TABLE maos_host_global.legal_holds ADD COLUMN hold_id BLOB;"
                    }
                })
                .map_err(AuditError::SqliteWriteFatal)?;
        }
        transaction.execute_batch(match authority {
            LegalHoldAuthority::Main => "UPDATE main.legal_holds SET hold_id=randomblob(16) WHERE hold_id IS NULL;
                CREATE TRIGGER IF NOT EXISTS main.legal_hold_identity AFTER INSERT ON legal_holds WHEN NEW.hold_id IS NULL
                BEGIN UPDATE legal_holds SET hold_id=randomblob(16) WHERE principal_id=NEW.principal_id; END;",
            LegalHoldAuthority::Attached => "UPDATE maos_host_global.legal_holds SET hold_id=randomblob(16) WHERE hold_id IS NULL;
                CREATE TRIGGER IF NOT EXISTS maos_host_global.legal_hold_identity AFTER INSERT ON legal_holds WHEN NEW.hold_id IS NULL
                BEGIN UPDATE legal_holds SET hold_id=randomblob(16) WHERE principal_id=NEW.principal_id; END;",
        }).map_err(AuditError::SqliteWriteFatal)?;
        transaction.commit().map_err(AuditError::SqliteWriteFatal)
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
             ON transparency_log(correlation_id, timestamp_ns);
             CREATE INDEX IF NOT EXISTS idx_tlog_boot_pid_ts
             ON transparency_log(boot_nonce, spirit_pid, timestamp_ns, frame_id);",
        )?;
        Ok(Self {
            erasure_holds: Mutex::new(()),
            inner: parking_lot::FairMutex::new(TransparencyLogInner {
                conn,
                next_frame_id_counter: 0,
                boot_nonce,
                last_frame_id: [0u8; 16],
                legal_hold_authority: None,
            }),
            redaction,
            mailbox: MailboxStub::new(),
            recipient_resolver: Default::default(),
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
        conn.execute_batch(
            "CREATE INDEX idx_tlog_correlation ON transparency_log(correlation_id, timestamp_ns)",
        )
        .expect("in-memory correlation index must match the file-backed holder");
        conn.execute_batch(LEGAL_HOLDS_TABLE_SQL)
            .expect("in-memory legal-hold schema init must succeed");
        Self {
            erasure_holds: Mutex::new(()),
            inner: parking_lot::FairMutex::new(TransparencyLogInner {
                conn,
                next_frame_id_counter: 0,
                boot_nonce,
                last_frame_id: [0u8; 16],
                legal_hold_authority: Some(LegalHoldAuthority::Main),
            }),
            redaction,
            mailbox: MailboxStub::new(),
            recipient_resolver: Default::default(),
        }
    }

    /// Boot incarnation bound to writes and local Spirit recall.
    pub fn boot_nonce(&self) -> u64 {
        self.inner.lock().boot_nonce
    }

    /// Operator-only MCP refusal receipt shared by issuance and use-time mediation.
    pub fn record_mcp_refusal(
        &self,
        pid: u32,
        spirit_id: Option<&str>,
        server: &str,
        tool: &str,
        code: &str,
    ) {
        // Field order is part of the receipt format.
        #[derive(serde::Serialize)]
        struct Payload<'a> {
            op: &'a str,
            code: &'a str,
            server: &'a str,
            tool: &'a str,
            boot_nonce: u64,
        }
        let payload = serde_json::to_vec(&Payload {
            op: "mcp.call",
            code,
            server,
            tool,
            boot_nonce: self.boot_nonce(),
        })
        .expect("MCP refusal receipt contains only serializable scalar fields");
        let _ = self.insert_frame_event_with_sender(
            FrameKind::GovernanceEvent,
            pid,
            spirit_id.unwrap_or(""),
            "",
            None,
            &format!("mcp.call.refused:{code}"),
            &payload,
            FrameOrigin::Kernel,
        );
    }

    /// Journal an authenticated MCP context refusal and keep its error typed.
    pub fn refuse_context_mcp(&self, pid: u32, server: &str, tool: &str) -> McpError {
        self.record_mcp_refusal(pid, None, server, tool, "context-exhausted");
        McpError::ContextExhausted {
            server: server.into(),
            tool: tool.into(),
        }
    }

    /// Generate a unique 16-byte frame ID using ULID.
    fn next_frame_id(counter: &mut u64) -> [u8; 16] {
        *counter += 1;
        let ulid = ulid::Ulid::new();
        let mut bytes = ulid.to_bytes();
        // Mix in the counter for additional uniqueness within the same millisecond
        let counter_bytes = counter.to_le_bytes();
        bytes[12] ^= counter_bytes[0];
        bytes[13] ^= counter_bytes[1];
        bytes[14] ^= counter_bytes[2];
        bytes[15] ^= counter_bytes[3];
        bytes
    }

    fn append_erasure_row(
        conn: &Connection,
        counter: &mut u64,
        boot: u64,
        kind: FrameKind,
        intent: &str,
        payload: &[u8],
    ) -> Result<[u8; 16], AuditError> {
        assert!(
            matches!(kind, FrameKind::TaskComplete | FrameKind::Decision),
            "erasure transaction writer is not a general frame-kind bypass"
        );
        let id = Self::next_frame_id(counter);
        conn.execute(
            "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
             VALUES(?1,?2,0,?3,?4,?5,?6,?7)",
            rusqlite::params![id.as_slice(), wall_clock_now_ns() as i64, boot as i64, kind as i64, intent, payload, FrameOrigin::Kernel as i64],
        ).map_err(AuditError::SqliteWriteFatal)?;
        Ok(id)
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
        // Kernel-minted `frame_id` (`None` below), so a duplicate here is a kernel
        // sequence collision, not a peer replay — it keeps halting (AC3.2 scope).
        if kind == FrameKind::Distillate {
            panic!(
                "MAOS I11 enforcement (Story 8.10 AC2): FrameKind::Distillate rows \
                 may only be inserted via DistillateWriter"
            );
        }
        Self::halt_on_duplicate(self.insert_frame_row_with_correlation(
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
            None,
        ))
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
        // Kernel-minted `frame_id` (`None`): a duplicate is a kernel sequence
        // collision, never a peer replay, so it keeps halting (AC3.2 scope wall).
        Self::halt_on_duplicate(
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
            .into_inner(),
        )
    }

    /// Insert a frame event with explicit frame ID and sender tracking.
    ///
    /// Use this when the frame ID is externally generated (e.g. an IAC frame
    /// that must be retrievable by its original frame_id for retraction).
    /// Pass `None` for `frame_id` to auto-generate one.
    ///
    /// **Returns [`FrameRowWrite`] inside the I2 typestate (j1-crosshost-2b
    /// AC3.2).** This is the one writer that accepts a **peer-supplied**
    /// `frame_id` on the cross-host path, and J1 ids are deterministic
    /// `seq ‖ run_nonce` with no ULID entropy — so a peer that re-sends one frame
    /// used to halt the receiving Host through the `panic!` in
    /// `insert_frame_row_with_correlation`. It became reachable the moment
    /// AC1.2 installed a production intake sink, which is why the repair landed in
    /// the same change. `Duplicate` means *the row is already in the log*: the I2
    /// guarantee holds, nothing was lost, and the caller decides idempotency.
    /// Callers that mint their own ids and cannot express a duplicate stay on
    /// [`Self::insert_frame_event_with_sender`], which still halts.
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
    ) -> LogBeforeDeliver<FrameRowWrite> {
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
        LogBeforeDeliver::new(self.insert_frame_row(
            frame_id,
            kind,
            spirit_pid,
            from_spirit_id,
            to_spirit_id,
            capability_token,
            intent,
            payload,
            origin,
        ))
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
        // I11 distillate ids are kernel/writer-minted, never peer-supplied, so a
        // duplicate stays a halt (AC3.2 scope wall).
        Self::halt_on_duplicate(self.insert_frame_row(
            frame_id,
            FrameKind::Distillate,
            spirit_pid,
            from_spirit_id,
            to_spirit_id,
            capability_token,
            intent,
            payload,
            origin,
        ))
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
    ) -> FrameRowWrite {
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
            None,
        )
    }

    /// Story 17-8 AC2.2 — the one routed-frame writer (`deliver_typed` direct and
    /// DRR, `retract` both arms): the base row, its recorded recipients (already
    /// deduped by pid, CR-1) and its consent envelope commit in ONE transaction.
    /// `Duplicate` writes and replaces nothing; any other failure rolls back and
    /// halts (I2). The base row is unchanged: `to_spirit_id` = first addressee.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_routed_frame(
        &self,
        frame: &maos_domain::frame::IacFrame,
        kind: FrameKind,
        spirit_pid: u32,
        capability_token: Option<&[u8; 32]>,
        intent: &str,
        payload: &[u8],
        recipients: &[FrameRecipient],
    ) -> LogBeforeDeliver<FrameRowWrite> {
        assert!(
            kind != FrameKind::Distillate,
            "MAOS I11 enforcement (Story 8.10 AC2): FrameKind::Distillate rows may only be \
             inserted via DistillateWriter"
        );
        LogBeforeDeliver::new(self.insert_frame_row_with_correlation(
            Some(frame.frame_id),
            kind,
            spirit_pid,
            frame.from.spirit_id.as_str(),
            frame.to.first().map_or("", |a| a.spirit_id.as_str()),
            capability_token,
            None,
            intent,
            payload,
            frame.auto_marker,
            Some(SideRows::Routed {
                recipients,
                consent: frame.consent_envelope.as_ref(),
            }),
        ))
    }

    /// Story 17-8 AC3.5 — journal a kind-22 rupture row at pid 0 and, given a
    /// `grant`, one grant-scoped rupture row per rejected recipient pid, in ONE
    /// transaction. A kernel-minted id (`None`) halts on a duplicate like
    /// [`Self::insert_frame_event_with_sender`]; a supplied id returns the typed
    /// outcome like [`Self::insert_frame_event_with_id`].
    #[allow(clippy::too_many_arguments)]
    pub fn insert_consent_rupture(
        &self,
        frame_id: Option<[u8; 16]>,
        from_spirit_id: &str,
        to_spirit_id: &str,
        intent: &str,
        payload: &[u8],
        origin: FrameOrigin,
        grant: Option<(&ConsentEnvelope, &[u32])>,
    ) -> LogBeforeDeliver<FrameRowWrite> {
        let write = self.insert_frame_row_with_correlation(
            frame_id,
            FrameKind::ConsentRupture,
            0,
            from_spirit_id,
            to_spirit_id,
            None,
            None,
            intent,
            payload,
            origin,
            grant.map(|(envelope, recipient_pids)| SideRows::Rupture {
                envelope,
                recipient_pids,
            }),
        );
        if frame_id.is_none() {
            let _ = Self::halt_on_duplicate(write);
        }
        LogBeforeDeliver::new(write)
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
        sides: Option<SideRows<'_>>,
    ) -> FrameRowWrite {
        let redacted = self.redaction.redact(payload);
        let mut inner = self.inner.lock();
        let frame_id =
            frame_id.unwrap_or_else(|| Self::next_frame_id(&mut inner.next_frame_id_counter));
        let timestamp_ns = wall_clock_now_ns();
        let boot_nonce = inner.boot_nonce;
        let transactional = sides.is_some();
        let write = |conn: &Connection| {
            match conn
                .prepare_cached(FRAME_ROW_INSERT_SQL)
                .and_then(|mut statement| {
                    statement.execute(rusqlite::params![
                        &frame_id[..],
                        timestamp_ns as i64,
                        spirit_pid as i64,
                        from_spirit_id,
                        to_spirit_id,
                        boot_nonce as i64,
                        capability_token.map(|token| &token[..]),
                        kind as i64,
                        intent,
                        correlation_id,
                        &redacted[..],
                        origin as i64,
                    ])
                }) {
                Ok(_) => sides
                    .map_or(Ok(()), |sides| sides.write(conn, &frame_id))
                    .map(|()| FrameRowWrite::Written),
                // Only the base row's primary key can mean already journaled.
                Err(error) if Self::is_duplicate_primary_key(&error) => {
                    Ok(FrameRowWrite::Duplicate)
                }
                Err(error) => Err(error),
            }
        };
        // Preserve IMMEDIATE + atomic side rows; RAII also rolls back unwinds.
        let result = if transactional {
            inner
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .and_then(|transaction| {
                    let outcome = write(&transaction)?;
                    match outcome {
                        FrameRowWrite::Written => transaction.commit()?,
                        FrameRowWrite::Duplicate => transaction.rollback()?,
                    }
                    Ok(outcome)
                })
        } else {
            write(&inner.conn)
        };
        match result {
            Ok(write) => {
                inner.last_frame_id = frame_id;
                write
            }
            Err(error) => {
                // This panic, before delivery, is the I2 failure boundary.
                // The fair mutex is not a poisoned-state authority.
                panic!(
                    "MAOS kernel panic — Transparency Log write failed: {error}. \
                     Architecture §7.3 I2: log-before-deliver guarantee broken; \
                     kernel halts. Audit the SQLite file for corruption."
                );
            }
        }
    }

    /// Is this write error EXACTLY a duplicate-primary-key (or unique-index)
    /// violation?
    ///
    /// j1-crosshost-2b AC3.2 — discriminate on the **extended** code, never on
    /// `rusqlite::ErrorCode::ConstraintViolation`. `ErrorCode` is a re-export of
    /// the ffi enum and `ConstraintViolation` is mapped from the *primary* code
    /// `SQLITE_CONSTRAINT`, which also covers `NOT NULL`, `CHECK` and
    /// `FOREIGN KEY`. `transparency_log` declares `NOT NULL` on **ten of its
    /// twelve columns** (only `capability_token` and `correlation_id` are
    /// nullable), so matching the primary code would silently reclassify a genuine
    /// NOT-NULL defect as "already journaled" — the exact inversion of this
    /// repair's intent. Pinned deps: `rusqlite 0.31` / `libsqlite3-sys 0.28`.
    /// `pub` so the extended-code discrimination itself is testable from
    /// `crates/maos-iac/tests/` (§A6 review P4: the AC3.2 negative — a planted
    /// NOT NULL violation must NOT classify as a duplicate — is the control
    /// that stops the next author from widening the arm back to
    /// `ErrorCode::ConstraintViolation`).
    pub fn is_duplicate_primary_key(error: &rusqlite::Error) -> bool {
        let rusqlite::Error::SqliteFailure(e, _) = error else {
            return false;
        };
        e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
            || e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    }

    /// Preserve the pre-j1-crosshost-2b halt for the writers whose `frame_id` is
    /// **kernel-minted** and therefore cannot legitimately collide: a duplicate
    /// there is a sequence defect in this process, not a peer replaying a
    /// deterministic cross-host id. AC3.2's scope wall in one function — only the
    /// peer-supplied path ([`Self::insert_frame_event_with_id`]) returns the typed
    /// outcome; everyone else still halts, and every non-duplicate write error
    /// halts everywhere.
    fn halt_on_duplicate(write: FrameRowWrite) -> LogBeforeDeliver<()> {
        assert!(
            write == FrameRowWrite::Written,
            "MAOS kernel panic — Transparency Log write failed: duplicate frame_id on a \
             KERNEL-MINTED id (Architecture §7.3 I2). Not a peer replay — that is handled typed \
             at insert_frame_event_with_id. Audit the SQLite file and the id source."
        );
        LogBeforeDeliver::new(())
    }

    /// Enumerate I13 carriers using full boot-local writer identities.
    pub fn distillates_written_by(
        &self,
        incarnations: &[(u64, u32)],
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        let mut found = Vec::new();
        self.scan_rowid_windows(
            "SELECT frame_id, boot_nonce, spirit_pid, payload_redacted, timestamp_ns FROM transparency_log INDEXED BY idx_tlog_distillate_rowid \
             WHERE rowid > ?2 AND rowid <= ?3 AND ?1 IS ?1 AND kind = 11", None,
            |row| {
                let boot = row.get::<_, i64>(1).map_err(AuditError::SqliteRead)? as u64;
                let pid = row.get::<_, i64>(2).map_err(AuditError::SqliteRead)? as u32;
                if !incarnations.contains(&(boot, pid)) { return Ok(()); }
                let body: Vec<u8> = row.get(3).map_err(AuditError::SqliteRead)?;
                if Self::is_kernel_tombstone(&body) { return Ok(()); }
                let bytes: Vec<u8> = row.get(0).map_err(AuditError::SqliteRead)?;
                found.push(ForgetFrame {
                    timestamp_ns: row.get(4).map_err(AuditError::SqliteRead)?,
                    frame_id: bytes.try_into().map_err(|v: Vec<u8>|AuditError::MalformedFrameId(v.len()))?,
                    kind: 11, spirit_pid: pid, boot_nonce: boot,
                    source_log_ref: Self::distillate_source_refs(&body).unwrap_or_default(),
                });
                Ok(())
            })?;
        found.sort_by_key(|frame| (frame.timestamp_ns, frame.frame_id));
        Ok(found)
    }

    fn protected_by_holds(
        &self,
        holds: &[PersistedLegalHold],
    ) -> Result<std::collections::HashSet<String>, AuditError> {
        self.protected_at(holds, self.frame_high_water()?)
    }

    fn protected_at(
        &self,
        holds: &[PersistedLegalHold],
        through: i64,
    ) -> Result<std::collections::HashSet<String>, AuditError> {
        if holds.is_empty() {
            return Ok(std::collections::HashSet::new());
        }
        let mut protected = std::collections::HashSet::new();
        for hold in holds {
            for frame in self.forget_frames(
                Some(&hold.principal_id),
                &std::collections::HashSet::new(),
                false,
                (0, through),
            )? {
                protected.insert(format!("{:032x}", u128::from_be_bytes(frame.frame_id)));
            }
        }
        for frame in self.distillate_closure_at(&protected, false, (0, through))? {
            protected.insert(format!("{:032x}", u128::from_be_bytes(frame.frame_id)));
        }
        Ok(protected)
    }

    /// Hold recheck and body mutation share one IMMEDIATE transaction, including
    /// the attached global authority. Independent connections cannot place a
    /// hold in the gap. The frozen kernel's later private/index deletion is a
    /// separately recorded coverage limitation, not covered by this TL lease.
    pub fn scrub_distillate_unless_held(
        &self,
        frame_id: [u8; 16],
        reason: &str,
    ) -> Result<bool, AuditError> {
        self.scrub_body_unless_held(frame_id, FrameKind::Distillate as i64, reason)
    }

    fn scrub_body_unless_held(
        &self,
        frame_id: [u8; 16],
        kind: i64,
        reason: &str,
    ) -> Result<bool, AuditError> {
        let id = format!("{:032x}", u128::from_be_bytes(frame_id));
        for _ in 0..8 {
            let holds = self.list_legal_holds()?;
            let protected = self.protected_by_holds(&holds)?;
            match self.write_erasure_body(
                ErasureTarget {
                    frame_id: &frame_id,
                    kind,
                    id: &id,
                    principals: &[],
                    spirit: None,
                    reason,
                    request: None,
                },
                &holds,
                &protected,
            )? {
                ErasureWrite::Retry => continue,
                ErasureWrite::Held => return Ok(false),
                ErasureWrite::Redacted => return Ok(true),
            }
        }
        Err(AuditError::ErasurePending)
    }

    fn write_erasure_body(
        &self,
        target: ErasureTarget<'_>,
        holds: &[PersistedLegalHold],
        protected: &std::collections::HashSet<String>,
    ) -> Result<ErasureWrite, AuditError> {
        let payload = target
            .request
            .map(|request| {
                let mut value = serde_json::json!({ "request": request });
                Self::redistill_metadata_ids(&mut value, true);
                serde_json::to_vec(&value)
                    .map_err(|error| AuditError::Serialization(error.to_string()))
            })
            .transpose()?;
        let payload = payload.as_ref().map(|bytes| self.redaction.redact(bytes));
        let _hold_guard = self
            .erasure_holds
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut inner = self.inner.lock();
        let authority = inner
            .legal_hold_authority
            .ok_or(AuditError::LegalHoldAuthorityUnbound)?;
        let TransparencyLogInner {
            conn,
            next_frame_id_counter,
            last_frame_id,
            boot_nonce,
            ..
        } = &mut *inner;
        let transaction = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(AuditError::SqliteWriteFatal)?;
        if Self::read_legal_holds(&transaction, authority)? != holds {
            return Ok(ErasureWrite::Retry);
        }
        #[cfg(test)]
        erasure_races_20_11::before_body_mutation();
        if protected.contains(target.id) {
            return Ok(ErasureWrite::Held);
        }
        let replacement = if matches!(target.kind, 28 | 29) {
            if target.principals.is_empty() {
                return Err(AuditError::Serialization(
                    "dimensional redaction requires a subject".into(),
                ));
            }
            let existing: Vec<u8> = transaction
                .query_row(
                    "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
                    [target.frame_id.as_slice()],
                    |row| row.get(0),
                )
                .map_err(AuditError::SqliteRead)?;
            Self::redacted_principals_body(
                target.kind,
                &existing,
                target.principals,
                target.reason,
                *target.frame_id,
            )?
        } else {
            let kind = if target.kind == 11 {
                serde_json::json!("Distillate")
            } else {
                serde_json::json!(target.kind)
            };
            serde_json::to_vec(&serde_json::json!({ "redacted": true, "reason": target.reason, "original_kind": kind }))
                .map_err(|error| AuditError::Serialization(error.to_string()))?
        };
        if let Some(payload) = payload.as_deref() {
            Self::append_erasure_row(
                &transaction,
                next_frame_id_counter,
                *boot_nonce,
                FrameKind::TaskComplete,
                "distillate.redistill.requested",
                payload,
            )?;
        }
        let changed = transaction
            .execute(
                "UPDATE transparency_log SET payload_redacted=?1 WHERE frame_id=?2 AND kind=?3",
                rusqlite::params![replacement, target.frame_id.as_slice(), target.kind],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        if changed != 1 {
            return Err(AuditError::Serialization(
                "erasure target disappeared or changed kind".into(),
            ));
        }
        let id = format_frame_id_hex(target.frame_id);
        let marker = match (target.kind, target.spirit, target.principals.first()) {
            (11, Some(spirit), _) => {
                serde_json::json!({"spirit_id":spirit,"redacted_distillate_frame_id":id})
            }
            (11, None, Some(principal)) if target.principals.len() == 1 => {
                serde_json::json!({"principal_id":principal,"redacted_distillate_frame_id":id})
            }
            _ => serde_json::json!({"redacted_frame_id":id,"original_kind":target.kind}),
        };
        let marker = serde_json::to_vec(&marker)
            .map_err(|error| AuditError::Serialization(error.to_string()))?;
        let marker_id = Self::append_erasure_row(
            &transaction,
            next_frame_id_counter,
            *boot_nonce,
            FrameKind::TaskComplete,
            if target.kind == 11 {
                "distillate.redacted"
            } else {
                "frame.redacted"
            },
            &marker,
        )?;
        transaction.commit().map_err(AuditError::SqliteWriteFatal)?;
        *last_frame_id = marker_id;
        Ok(ErasureWrite::Redacted)
    }

    /// Story 9.2 — return every frame_id in the Transparency Log, sorted.
    pub fn all_frame_ids(&self) -> Result<Vec<[u8; 16]>, AuditError> {
        let inner = self.inner.lock();
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
        let inner = self.inner.lock();
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

    /// Any writer, any boot, any kind, with token-bounded confirmation.
    pub fn principal_bearing_frames(
        &self,
        principal: &str,
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        self.forget_frames(
            Some(principal),
            &std::collections::HashSet::new(),
            false,
            (0, self.frame_high_water()?),
        )
    }

    /// Close direct and flattened derivation transitively, including prior tombstones.
    pub fn distillates_citing(
        &self,
        scrubbed: &std::collections::HashSet<String>,
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        self.distillate_closure(scrubbed, true)
    }

    fn distillate_closure(
        &self,
        scrubbed: &std::collections::HashSet<String>,
        include_prior: bool,
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        self.distillate_closure_at(scrubbed, include_prior, (0, self.frame_high_water()?))
    }

    fn distillate_closure_at(
        &self,
        scrubbed: &std::collections::HashSet<String>,
        include_prior: bool,
        range: (i64, i64),
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        self.closure_with_prior(
            scrubbed,
            include_prior,
            range,
            &self.prior_tombstones((0, range.1))?,
        )
    }

    fn closure_with_prior(
        &self,
        scrubbed: &std::collections::HashSet<String>,
        include_prior: bool,
        range: (i64, i64),
        prior: &std::collections::HashSet<String>,
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        if scrubbed.is_empty() && (!include_prior || prior.is_empty()) {
            return Ok(Vec::new());
        }
        // One metadata scan, then O(frames + edges) graph work. Re-imported
        // direct-parent histories need not be in topological rowid order.
        let frames = self.forget_candidates(None, None, false, range, prior)?;
        let mut children: std::collections::HashMap<&str, Vec<usize>> =
            std::collections::HashMap::new();
        for (index, frame) in frames.iter().enumerate() {
            for source in &frame.source_log_ref {
                children.entry(source).or_default().push(index);
            }
        }
        let mut queue = std::collections::VecDeque::new();
        for source in scrubbed
            .iter()
            .chain(prior.iter().filter(|_| include_prior))
        {
            if let Some(indices) = children.get(source.as_str()) {
                queue.extend(indices.iter().copied());
            }
        }
        let mut reached = vec![0_u8; frames.len()];
        while let Some(index) = queue.pop_front() {
            if reached[index] != 0 {
                continue;
            }
            let id = format!("{:032x}", u128::from_be_bytes(frames[index].frame_id));
            reached[index] = if scrubbed.contains(&id) { 1 } else { 2 };
            if let Some(indices) = children.get(id.as_str()) {
                queue.extend(indices.iter().copied());
            }
        }
        drop(children);
        let mut found = frames;
        let mut index = 0;
        found.retain_mut(|frame| {
            let keep = reached[index] == 2;
            index += 1;
            if keep {
                frame.source_log_ref.retain(|id| !prior.contains(id));
            }
            keep
        });
        Ok(found)
    }

    /// Pin entry high-water once. Appends belong to a separate bounded tail
    /// sweep, never an open-ended extension of this finite scan.
    fn scan_rowid_windows(
        &self,
        sql: &str,
        principal: Option<&str>,
        each: impl FnMut(&rusqlite::Row<'_>) -> Result<(), AuditError>,
    ) -> Result<(), AuditError> {
        self.scan_rowid_range(sql, principal, (0, self.frame_high_water()?), each)
    }

    /// Constant-work presence query, including histories with imported rowids.
    pub fn has_frames(&self) -> Result<bool, AuditError> {
        let inner = self.inner.lock();
        inner
            .conn
            .query_row("SELECT EXISTS(SELECT 1 FROM transparency_log)", [], |row| {
                row.get(0)
            })
            .map_err(AuditError::SqliteRead)
    }

    fn frame_high_water(&self) -> Result<i64, AuditError> {
        self.inner
            .lock()
            .conn
            .query_row(
                "SELECT coalesce(max(rowid),0) FROM transparency_log",
                [],
                |row| row.get(0),
            )
            .map_err(AuditError::SqliteRead)
    }

    fn scan_rowid_range(
        &self,
        sql: &str,
        principal: Option<&str>,
        range: (i64, i64),
        mut each: impl FnMut(&rusqlite::Row<'_>) -> Result<(), AuditError>,
    ) -> Result<(), AuditError> {
        const FORGET_SCAN_PAGE: i64 = 4096;
        let mut after = range.0;
        while after < range.1 {
            let inner = self.inner.lock();
            // Count actual rows, not rowid distance: sparse legacy rowids must
            // not create billions of empty windows or enlarge a lock window.
            let mut boundary = inner.conn.prepare_cached(
                "SELECT coalesce(max(rowid),?1) FROM
                 (SELECT rowid FROM transparency_log WHERE rowid>?1 AND rowid<=?2 ORDER BY rowid LIMIT ?3)",
            ).map_err(AuditError::SqliteRead)?;
            let upto: i64 = boundary
                .query_row(rusqlite::params![after, range.1, FORGET_SCAN_PAGE], |row| {
                    row.get(0)
                })
                .map_err(AuditError::SqliteRead)?;
            #[cfg(test)]
            let boundary_steps = boundary.reset_status(rusqlite::StatementStatus::VmStep);
            drop(boundary);
            if upto == after {
                return Ok(());
            }
            let mut stmt = inner
                .conn
                .prepare_cached(sql)
                .map_err(AuditError::SqliteRead)?;
            let mut rows = stmt
                .query(rusqlite::params![principal, after, upto])
                .map_err(AuditError::SqliteRead)?;
            while let Some(row) = rows.next().map_err(AuditError::SqliteRead)? {
                each(row)?;
            }
            after = upto;
            drop(rows);
            #[cfg(test)]
            erasure_scan_work::record(&stmt, sql, boundary_steps);
            drop(stmt);
            drop(inner);
            #[cfg(test)]
            erasure_races_20_11::after_window(self, sql, principal);
            // FairMutex hands the next lock turn to an already-queued writer.
        }
        Ok(())
    }

    fn forget_frames(
        &self,
        principal: Option<&str>,
        scrubbed: &std::collections::HashSet<String>,
        include_prior: bool,
        range: (i64, i64),
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        let prior = self.prior_tombstones((0, range.1))?;
        self.forget_candidates(principal, Some(scrubbed), include_prior, range, &prior)
    }

    fn prior_tombstones(
        &self,
        range: (i64, i64),
    ) -> Result<std::collections::HashSet<String>, AuditError> {
        let mut prior = std::collections::HashSet::new();
        self.scan_rowid_range(
            "SELECT lower(hex(frame_id)) FROM transparency_log NOT INDEXED \
             WHERE rowid > ?2 AND rowid <= ?3 AND ?1 IS ?1 AND \
             CASE WHEN json_valid(CAST(payload_redacted AS TEXT)) \
             THEN json_extract(CAST(payload_redacted AS TEXT), '$.redacted') = 1 \
             AND (json_type(CAST(payload_redacted AS TEXT), '$.reason') = 'text' \
             AND (json_type(CAST(payload_redacted AS TEXT), '$.original_kind') = 'integer' OR json_extract(CAST(payload_redacted AS TEXT), '$.original_kind') = 'Distillate') \
             AND (SELECT count(*) FROM json_each(CAST(payload_redacted AS TEXT))) = 3) ELSE 0 END",
            None,
            range,
            |row| { prior.insert(row.get::<_, String>(0).map_err(AuditError::SqliteRead)?); Ok(()) },
        )?;
        Ok(prior)
    }

    fn forget_candidates(
        &self,
        principal: Option<&str>,
        scrubbed: Option<&std::collections::HashSet<String>>,
        include_prior: bool,
        range: (i64, i64),
        prior: &std::collections::HashSet<String>,
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        let mut found = Vec::new();
        let sql = if principal.is_none() {
            "SELECT frame_id,kind,spirit_pid,boot_nonce,payload_redacted,timestamp_ns \
             FROM transparency_log INDEXED BY idx_tlog_distillate_rowid \
             WHERE rowid>?2 AND rowid<=?3 AND ?1 IS ?1 AND kind=11"
        } else {
            "SELECT frame_id,kind,spirit_pid,boot_nonce,payload_redacted,timestamp_ns \
             FROM transparency_log NOT INDEXED WHERE rowid>?2 AND rowid<=?3 \
             AND (kind IN (28,29) OR instr(payload_redacted,CAST(?1 AS BLOB))>0) \
             AND intent NOT IN ('principal.forget','principal.forget.held','distillate.redacted') \
             AND intent NOT LIKE 'principal.uninstall.%'"
        };
        self.scan_rowid_range(sql, principal, range, |row| {
            let body: Vec<u8> = row.get(4).map_err(AuditError::SqliteRead)?;
            let kind: i64 = row.get(1).map_err(AuditError::SqliteRead)?;
            if principal.is_some_and(|id| !Self::subject_in_frame(kind, &body, id)) {
                return Ok(());
            }
            let mut source_log_ref = if kind == FrameKind::Distillate as i64 {
                Self::distillate_source_refs(&body).unwrap_or_default()
            } else {
                Vec::new()
            };
            if principal.is_none()
                && match scrubbed {
                    Some(scrubbed) => !source_log_ref
                        .iter()
                        .any(|id| scrubbed.contains(id) || (include_prior && prior.contains(id))),
                    None => source_log_ref.is_empty(),
                }
            {
                return Ok(());
            }
            if scrubbed.is_some() {
                source_log_ref.retain(|id| !prior.contains(id));
            }
            let bytes: Vec<u8> = row.get(0).map_err(AuditError::SqliteRead)?;
            found.push(ForgetFrame {
                timestamp_ns: row.get(5).map_err(AuditError::SqliteRead)?,
                frame_id: bytes
                    .try_into()
                    .map_err(|v: Vec<u8>| AuditError::MalformedFrameId(v.len()))?,
                kind,
                spirit_pid: row.get::<_, i64>(2).map_err(AuditError::SqliteRead)? as u32,
                boot_nonce: row.get::<_, i64>(3).map_err(AuditError::SqliteRead)? as u64,
                source_log_ref,
            });
            Ok(())
        })?;
        found.sort_by_key(|frame| (frame.timestamp_ns, frame.frame_id));
        Ok(found)
    }

    /// Shared first/late sweep; held co-principals protect their entire derivation.
    pub fn scrub_forget_frames(
        &self,
        principal: &str,
    ) -> Result<
        (
            Vec<String>,
            Vec<String>,
            Vec<RedistillRequest>,
            Vec<String>,
            Vec<String>,
        ),
        AuditError,
    > {
        let mut state = ErasureSweep::new(vec![principal.to_owned()], None);
        let mut settled = false;
        for _ in 0..8 {
            self.sweep_erasure(&mut state)?;
            if state.blocked() || self.certify_frame_scope(&mut state, false)? {
                settled = true;
                break;
            }
        }
        if !settled {
            return Err(AuditError::ErasurePending);
        }
        if !state.held_principals.is_empty() {
            return Err(AuditError::ErasurePending);
        }
        state.distillates.sort_unstable();
        state.held.sort_unstable();
        state.malformed.sort_unstable();
        state.principal_frames.sort_unstable();
        Ok((
            state.distillates.into_iter().map(|(_, id)| id).collect(),
            state
                .principal_frames
                .into_iter()
                .map(|(_, id)| id)
                .collect(),
            state.requests,
            state.held.into_iter().map(|(_, id)| id).collect(),
            state.malformed.into_iter().map(|(_, id)| id).collect(),
        ))
    }

    // Keep the CASE expressions identical to the expression index. The second
    // point lookup preserves malformed-outcome read errors without making an
    // otherwise valid opaque TL insert panic during index maintenance.
    /// Durable requests lacking any recorded outcome, for restart recovery.
    pub fn outstanding_redistill_requests(&self) -> Result<Vec<RedistillRequest>, AuditError> {
        let mut found = Vec::new();
        self.scan_rowid_windows(
            "SELECT r.payload_redacted, r.timestamp_ns, r.frame_id \
             FROM transparency_log r INDEXED BY idx_tlog_redistill_request_rowid \
             WHERE r.rowid > ?2 AND r.rowid <= ?3 AND ?1 IS ?1 \
             AND r.intent = 'distillate.redistill.requested' \
             AND NOT EXISTS (SELECT 1 FROM transparency_log o INDEXED BY idx_tlog_redistill_outcome \
               WHERE o.intent IN ('distillate.redistilled','distillate.redistill.pending') \
               AND CASE WHEN json_valid(CAST(o.payload_redacted AS TEXT),2) \
                 THEN json_extract(CAST(o.payload_redacted AS TEXT),'$.request.redacted_distillate_frame_id') \
                 ELSE X'' END = json_extract(CAST(r.payload_redacted AS TEXT),'$.request.redacted_distillate_frame_id') \
               UNION ALL SELECT 1 FROM transparency_log bad INDEXED BY idx_tlog_redistill_outcome \
               WHERE bad.intent IN ('distillate.redistilled','distillate.redistill.pending') \
               AND CASE WHEN json_valid(CAST(bad.payload_redacted AS TEXT),2) \
                 THEN json_extract(CAST(bad.payload_redacted AS TEXT),'$.request.redacted_distillate_frame_id') \
                 ELSE X'' END = X'' \
               AND json_extract(CAST(bad.payload_redacted AS TEXT),'$') IS NULL)",
            None,
            |row| {
                let body: Vec<u8> = row.get(0).map_err(AuditError::SqliteRead)?;
                let mut value: serde_json::Value = serde_json::from_slice(&body)
                    .map_err(|e| AuditError::Serialization(e.to_string()))?;
                Self::redistill_metadata_ids(&mut value, false);
                let request: RedistillRequest = serde_json::from_value(value["request"].take())
                    .map_err(|e| AuditError::Serialization(e.to_string()))?;
                let bytes: Vec<u8> = row.get(2).map_err(AuditError::SqliteRead)?;
                let id: [u8; 16] = bytes.try_into()
                    .map_err(|bytes: Vec<u8>| AuditError::MalformedFrameId(bytes.len()))?;
                found.push((row.get::<_, i64>(1).map_err(AuditError::SqliteRead)?, id, request));
                Ok(())
            },
        )?;
        found.sort_by_key(|(timestamp, id, _)| (*timestamp, *id));
        Ok(found.into_iter().map(|(_, _, request)| request).collect())
    }

    /// Only the exact kernel full-body replacement is an erased source.
    pub fn is_kernel_tombstone(body: &[u8]) -> bool {
        let Ok(serde_json::Value::Object(value)) = serde_json::from_slice(body) else {
            return false;
        };
        value.len() == 3
            && value.get("redacted").and_then(|v| v.as_bool()) == Some(true)
            && value.get("reason").is_some_and(|v| v.is_string())
            && value
                .get("original_kind")
                .is_some_and(|v| v.is_i64() || v.as_str() == Some("Distillate"))
    }

    /// Canonical plain-hex ids, shared with forget receipts and notices.
    pub fn distillate_source_refs(body: &[u8]) -> Result<Vec<String>, AuditError> {
        let value: serde_json::Value =
            serde_json::from_slice(body).map_err(|e| AuditError::Serialization(e.to_string()))?;
        if Self::is_kernel_tombstone(body) {
            return Ok(Vec::new());
        }
        let refs = value
            .get("source_log_ref")
            .and_then(|v| v.as_array())
            .ok_or_else(|| AuditError::Serialization("missing distillate source_log_ref".into()))?;
        refs.iter()
            .chain(
                value
                    .get("direct_source_log_ref")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten(),
            )
            .map(|value| {
                let id = value
                    .as_str()
                    .ok_or_else(|| AuditError::Serialization("non-string source id".into()))?;
                let id = id.replace(':', "");
                if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(AuditError::Serialization("invalid source frame id".into()));
                }
                Ok(id.to_ascii_lowercase())
            })
            .collect::<Result<std::collections::BTreeSet<_>, _>>()
            .map(|ids| ids.into_iter().collect())
    }

    /// Other frame kinds lose their whole payload; the marker contains no subject.
    pub fn scrub_frame_body(
        &self,
        frame_id: [u8; 16],
        kind: i64,
        reason: &str,
    ) -> Result<(), AuditError> {
        self.scrub_body_unless_held(frame_id, kind, reason)?
            .then_some(())
            .ok_or(AuditError::ErasurePending)
    }

    // Match DistillateWriter's colon-hex metadata: raw 32-hex ids look like tokens.
    fn redistill_metadata_ids(value: &mut serde_json::Value, durable: bool) {
        match value {
            serde_json::Value::String(id) => {
                if !durable {
                    id.retain(|c| c != ':');
                } else if id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()) {
                    let mut encoded = String::with_capacity(39);
                    for (i, c) in id.chars().enumerate() {
                        if i > 0 && i % 4 == 0 {
                            encoded.push(':');
                        }
                        encoded.push(c);
                    }
                    *id = encoded;
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    Self::redistill_metadata_ids(item, durable);
                }
            }
            serde_json::Value::Object(fields) => {
                for item in fields.values_mut() {
                    Self::redistill_metadata_ids(item, durable);
                }
            }
            _ => {}
        }
    }

    pub fn insert_redistill_request(&self, request: &RedistillRequest) -> Result<(), AuditError> {
        let mut value = serde_json::json!({"request": request});
        Self::redistill_metadata_ids(&mut value, true);
        let payload =
            serde_json::to_vec(&value).map_err(|e| AuditError::Serialization(e.to_string()))?;
        self.insert_frame_event(
            FrameKind::TaskComplete,
            0,
            None,
            "distillate.redistill.requested",
            &payload,
            FrameOrigin::Kernel,
        );
        Ok(())
    }

    pub fn insert_redistill_outcome(
        &self,
        request: &RedistillRequest,
        outcome: &RedistillOutcome,
    ) -> Result<(), AuditError> {
        let intent = match outcome {
            RedistillOutcome::Redistilled { .. } => "distillate.redistilled",
            RedistillOutcome::Pending { .. } => "distillate.redistill.pending",
        };
        let mut value = serde_json::json!({"request": request, "outcome": outcome});
        Self::redistill_metadata_ids(&mut value, true);
        let payload =
            serde_json::to_vec(&value).map_err(|e| AuditError::Serialization(e.to_string()))?;
        self.insert_frame_event(
            FrameKind::TaskComplete,
            0,
            None,
            intent,
            &payload,
            FrameOrigin::Kernel,
        );
        Ok(())
    }

    pub fn subject_in_frame(kind: i64, body: &[u8], subject: &str) -> bool {
        match matches!(kind, 28 | 29).then(|| serde_json::from_slice(body)) {
            Some(Ok(mut value)) => Self::scrub(kind, &mut value, &[subject]),
            _ => principal_id_occurs(body, subject),
        }
    }

    fn is_digest(text: &str) -> bool {
        text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
    }

    /// The one erasure predicate for kinds 28 and 29 (discovery, replacement, certification).
    /// Typed governance fields bound to their envelope and valid value form (the tag, the ABI
    /// ratification status, the canonical manifest fingerprint and publisher key) are set aside and
    /// restored; everything else is value-scrubbed by `walk`. Returns whether a value matched.
    fn scrub(kind: i64, v: &mut serde_json::Value, who: &[impl AsRef<str>]) -> bool {
        use maos_domain::governance::decode_declaration_bytes as decode;
        const TAGS: &str = "AbiExtension VetterKey SchemaLifecycle ModelProvenance TelemetrySubscription ManifestDeclaration";
        const PATHS: &str =
            "/event/governance_type /event/status /event/manifest_sha256 /event/publisher";
        let at = |p: &str| v.pointer(p).and_then(|t| t.as_str()).unwrap_or_default();
        let tag = at("/event/governance_type");
        let canonical = |p| tag == "ManifestDeclaration" && decode(at(p)).is_some();
        let allowed = [
            TAGS.split(' ').any(|t| t == tag),
            tag == "AbiExtension"
                && ["proposed", "ratified", "rejected"].contains(&at("/event/status")),
            canonical("/event/manifest_sha256"),
            canonical("/event/publisher"),
        ];
        let held: Vec<_> = PATHS
            .split(' ')
            .zip(allowed)
            .filter(|(_, ok)| *ok && kind == 28)
            .filter_map(|(p, _)| v.pointer_mut(p).map(|slot| (p, std::mem::take(slot))))
            .collect();
        let hit = Self::walk(kind, v, who, "");
        held.into_iter().for_each(|(p, held)| {
            v.pointer_mut(p)
                .map(|slot| *slot = held)
                .unwrap_or_default()
        });
        hit
    }

    /// Value-only erasure: object keys never change; kind 29 reaches only identity and label
    /// values, kind 28 every value except fixed receipt fields and canonical digests. Only the
    /// cost frame's `principal_id` and `task_ref` (`key`) are codec-encoded and compared decoded;
    /// every other string, including provider/model labels, is compared as written.
    fn walk(kind: i64, v: &mut serde_json::Value, w: &[impl AsRef<str>], key: &str) -> bool {
        use maos_domain::cost::decode_identity as plain;
        use maos_domain::memory::principal_id_occurs as seen;
        use serde_json::Value::{Array, Object, String as Text};
        const LABELS: &str = "principal principal_id task_ref session_principal provider model";
        const FIXED: &str = "event kernel_verdict trusted_intent trusted_origin";
        const DIGESTS: &str = "sha256 supersedes schema_content_hash";
        let listed = |names: &str, key: &str| names.split(' ').any(|name| name == key);
        let coded = kind == 29 && listed("principal_id task_ref", key);
        let text = |s| if coded { plain(s) } else { s.into() };
        let mut hit = false;
        match v {
            Text(s) if w.iter().any(|p| seen(text(s).as_bytes(), p.as_ref())) => {
                *s = "[REDACTED]".into();
                hit = true;
            }
            Array(a) => a.iter_mut().for_each(|i| hit |= Self::walk(kind, i, w, "")),
            Object(m) => m
                .iter_mut()
                .filter(|(k, i)| match kind {
                    29 => listed(LABELS, k),
                    _ => !i.as_str().is_some_and(|t| {
                        listed(FIXED, k) || listed(DIGESTS, k) && Self::is_digest(t)
                    }),
                })
                .for_each(|(k, i)| hit |= Self::walk(kind, i, w, k)),
            _ => {}
        }
        hit
    }

    fn redacted_principals_body(
        kind: i64,
        existing: &[u8],
        principals: &[String],
        reason: &str,
        frame_id: [u8; 16],
    ) -> Result<Vec<u8>, AuditError> {
        match serde_json::from_slice::<serde_json::Value>(existing) {
            Ok(mut val) => {
                Self::scrub(kind, &mut val, principals);
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("redacted".into(), true.into());
                    obj.insert("redaction_reason".into(), reason.into());
                }
                serde_json::to_vec(&val).map_err(|e| AuditError::Serialization(e.to_string()))
            }
            // Cannot parse payload: fall back to the full replacement tombstone.
            Err(_) => Ok(serde_json::json!({"redacted": true, "reason": reason, "original_frame_id": format_frame_id_hex(&frame_id)}).to_string().into_bytes()),
        }
    }

    /// Keep the principal-global legal-hold table on the Host-global artifact
    /// while this adapter writes frame/approval rows to a team shard.
    ///
    /// Legal-hold methods select the attached schema explicitly; an adapter
    /// without this binding returns `LegalHoldAuthorityUnbound`.
    pub fn attach_global_legal_holds(&self, global_path: &Path) -> Result<(), AuditError> {
        let mut inner = self.inner.lock();
        let same_artifact = inner.conn.path().is_some_and(|main| {
            matches!((Path::new(main).canonicalize(), global_path.canonicalize()),
                (Ok(main), Ok(global)) if main == global)
        });
        if same_artifact {
            drop(inner);
            return self.bind_main_legal_holds();
        }
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
                    requested_at_ns INTEGER NOT NULL,
                    hold_id BLOB NOT NULL DEFAULT (randomblob(16))
                 );",
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        Self::ensure_hold_identity(&mut inner.conn, LegalHoldAuthority::Attached)?;
        inner.legal_hold_authority = Some(LegalHoldAuthority::Attached);
        Ok(())
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
        let _hold_guard = self.erasure_holds.lock().unwrap_or_else(|e| e.into_inner());
        let inner = self.inner.lock();
        let sql = match inner
            .legal_hold_authority
            .ok_or(AuditError::LegalHoldAuthorityUnbound)?
        {
            LegalHoldAuthority::Main => {
                "INSERT OR REPLACE INTO main.legal_holds
                 (principal_id, reason, case_ref, requested_at_ns)
                 VALUES (?1, ?2, ?3, ?4)"
            }
            LegalHoldAuthority::Attached => {
                "INSERT OR REPLACE INTO maos_host_global.legal_holds
                 (principal_id, reason, case_ref, requested_at_ns)
                 VALUES (?1, ?2, ?3, ?4)"
            }
        };
        inner
            .conn
            .execute(
                sql,
                rusqlite::params![principal_id, reason, case_ref, requested_at_ns as i64,],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        Ok(())
    }

    /// Story 9.2 (P29) — is this principal under a durable legal hold?
    pub fn is_under_legal_hold(&self, principal_id: &str) -> Result<bool, AuditError> {
        let inner = self.inner.lock();
        let sql = match inner
            .legal_hold_authority
            .ok_or(AuditError::LegalHoldAuthorityUnbound)?
        {
            LegalHoldAuthority::Main => {
                "SELECT COUNT(*) FROM main.legal_holds WHERE principal_id = ?1"
            }
            LegalHoldAuthority::Attached => {
                "SELECT COUNT(*) FROM maos_host_global.legal_holds WHERE principal_id = ?1"
            }
        };
        let exists: i64 = inner
            .conn
            .query_row(sql, rusqlite::params![principal_id], |row| row.get(0))
            .map_err(AuditError::SqliteRead)?;
        Ok(exists > 0)
    }

    /// Release only the exact hold observed on entry. An independent process
    /// replacing even identical public fields creates a new identity (ABA-safe).
    pub fn release_legal_hold(&self, principal_id: &str) -> Result<bool, AuditError> {
        let _hold_guard = self
            .erasure_holds
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut inner = self.inner.lock();
        let (observe, remove) = match inner
            .legal_hold_authority
            .ok_or(AuditError::LegalHoldAuthorityUnbound)?
        {
            LegalHoldAuthority::Main => (
                "SELECT hold_id FROM main.legal_holds WHERE principal_id=?1",
                "DELETE FROM main.legal_holds WHERE principal_id=?1 AND hold_id=?2",
            ),
            LegalHoldAuthority::Attached => (
                "SELECT hold_id FROM maos_host_global.legal_holds WHERE principal_id=?1",
                "DELETE FROM maos_host_global.legal_holds WHERE principal_id=?1 AND hold_id=?2",
            ),
        };
        let observed = inner
            .conn
            .query_row(observe, [principal_id], |row| row.get::<_, HoldIdentity>(0))
            .optional()
            .map_err(AuditError::SqliteRead)?;
        let Some(observed) = observed else {
            return Ok(false);
        };
        #[cfg(test)]
        erasure_races_20_11::before_hold_delete();
        let transaction = inner
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(AuditError::SqliteWriteFatal)?;
        let removed = transaction
            .execute(
                remove,
                rusqlite::params![principal_id, observed.0.as_slice()],
            )
            .map_err(AuditError::SqliteWriteFatal)?;
        transaction.commit().map_err(AuditError::SqliteWriteFatal)?;
        Ok(removed != 0)
    }

    pub fn list_legal_holds(&self) -> Result<Vec<PersistedLegalHold>, AuditError> {
        let inner = self.inner.lock();
        Self::read_legal_holds(
            &inner.conn,
            inner
                .legal_hold_authority
                .ok_or(AuditError::LegalHoldAuthorityUnbound)?,
        )
    }

    fn read_legal_holds(
        conn: &Connection,
        authority: LegalHoldAuthority,
    ) -> Result<Vec<PersistedLegalHold>, AuditError> {
        let sql = match authority {
            LegalHoldAuthority::Main => {
                "SELECT principal_id, reason, case_ref, requested_at_ns
                 FROM main.legal_holds ORDER BY requested_at_ns, principal_id"
            }
            LegalHoldAuthority::Attached => {
                "SELECT principal_id, reason, case_ref, requested_at_ns
                 FROM maos_host_global.legal_holds ORDER BY requested_at_ns, principal_id"
            }
        };
        let mut statement = conn.prepare(sql).map_err(AuditError::SqliteRead)?;
        let rows = statement
            .query_map([], |row| {
                let requested_at_ns: i64 = row.get(3)?;
                Ok(PersistedLegalHold {
                    principal_id: row.get(0)?,
                    reason: row.get(1)?,
                    case_ref: row.get(2)?,
                    requested_at_ns: requested_at_ns.max(0) as u64,
                })
            })
            .map_err(AuditError::SqliteRead)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AuditError::SqliteRead)
    }

    /// Journal a caller-classified kernel frame and return its frame id under
    /// the same lock acquisition. The caller supplies both the semantic kind
    /// and complete 32-byte capability token; this path never invents either.
    /// Panics on write failure per the I2 binding, like `insert_frame_event`.
    pub fn insert_kernel_event_returning_id(
        &self,
        spirit_pid: u32,
        kind: FrameKind,
        capability_token: Option<[u8; 32]>,
        intent: &str,
        payload: &[u8],
    ) -> [u8; 16] {
        let redacted = self.redaction.redact(payload);
        let mut inner = self.inner.lock();
        let frame_id = Self::next_frame_id(&mut inner.next_frame_id_counter);
        let timestamp_ns = wall_clock_now_ns();
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
                capability_token.as_ref().map(|token| &token[..]),
                kind as i64,
                intent,
                &redacted[..],
                FrameOrigin::Kernel as i64,
            ],
        );
        match result {
            Ok(_) => {
                inner.last_frame_id = frame_id;
                frame_id
            }
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
        let inner = self.inner.lock();
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
    /// Returns entries in `(timestamp_ns, frame_id)` order, ascending by
    /// default and descending when [`FrameFilter::descending`] is set.
    pub fn query_frames(
        &self,
        filter: FrameFilter,
    ) -> Result<Vec<TransparencyLogEntry>, AuditError> {
        let inner = self.inner.lock();
        let (sql, params) = Self::frame_query(
            filter,
            "t.rowid, frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
             boot_nonce, capability_token, kind, intent, correlation_id, payload_redacted, origin",
        );

        let mut stmt = inner.conn.prepare(&sql).map_err(AuditError::SqliteRead)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let frame_id_blob: Vec<u8> = row.get(1)?;
                let mut frame_id = [0u8; 16];
                if frame_id_blob.len() == 16 {
                    frame_id.copy_from_slice(&frame_id_blob);
                }
                let cap_blob: Option<Vec<u8>> = row.get(7)?;
                let mut cap_token = None;
                if let Some(ref blob) = cap_blob {
                    if blob.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(blob);
                        cap_token = Some(arr);
                    }
                }
                Ok(TransparencyLogEntry {
                    insertion_id: row.get(0)?,
                    frame_id,
                    timestamp_ns: row.get::<_, i64>(2)? as u64,
                    spirit_pid: row.get::<_, i64>(3)? as u32,
                    from_spirit_id: row.get(4)?,
                    to_spirit_id: row.get(5)?,
                    boot_nonce: row.get::<_, i64>(6)? as u64,
                    capability_token: cap_token,
                    kind: FrameKind::from_i64(row.get::<_, i64>(8)?).unwrap_or_else(|| {
                        let disc = row.get::<_, i64>(8).unwrap_or(-1);
                        eprintln!(
                            "TL query: unrecognized FrameKind discriminant ({disc}); \
                                 mapping to TaskAssign as best-effort fallback \
                                 (schema-migration or cross-version log inspection)"
                        );
                        FrameKind::TaskAssign
                    }),
                    intent: row.get(9)?,
                    correlation_id: row.get(10)?,
                    payload_redacted: row.get(11)?,
                    origin: match row.get::<_, i64>(12)? {
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

    /// Query metadata without selecting or materializing any payload blobs.
    pub fn query_frame_metadata(
        &self,
        filter: FrameFilter,
    ) -> Result<Vec<TransparencyLogMetadata>, AuditError> {
        let inner = self.inner.lock();
        // Story 17-8 AC3.3 — a header-only row the caller only RECEIVED lists
        // with stored length 0 (`payload_available = false`); no blob is read.
        let projection = match filter.participant {
            Some(_) => format!(
                "frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, \
                 CASE WHEN t.spirit_pid <> caller.pid AND {HEADER_ONLY_PREDICATE} \
                 THEN 0 ELSE length(payload_redacted) END"
            ),
            None => FRAME_METADATA_PROJECTION.to_string(),
        };
        let (sql, params) = Self::frame_query(filter, &projection);
        let mut stmt = inner.conn.prepare(&sql).map_err(AuditError::SqliteRead)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let blob: Vec<u8> = row.get(0)?;
                let frame_id = blob.try_into().map_err(|blob: Vec<u8>| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(AuditError::MalformedFrameId(blob.len())),
                    )
                })?;
                Ok(TransparencyLogMetadata {
                    frame_id,
                    timestamp_ns: row.get::<_, i64>(1)? as u64,
                    spirit_pid: row.get::<_, i64>(2)? as u32,
                    boot_nonce: row.get::<_, i64>(3)? as u64,
                    raw_kind: row.get(4)?,
                    intent: row.get(5)?,
                    payload_len: row.get::<_, i64>(6)? as u64,
                })
            })
            .map_err(AuditError::SqliteRead)?;
        rows.map(|row| row.map_err(AuditError::SqliteRead))
            .collect()
    }

    /// Read raw kind and stored payload length by id, without loading the blob.
    /// Fetch callers must scope-check this metadata and enforce their byte bound
    /// before querying the payload.
    pub fn query_frame_metadata_by_id(
        &self,
        frame_id: [u8; 16],
    ) -> Result<Option<TransparencyLogMetadata>, AuditError> {
        Ok(self
            .query_frame_metadata(FrameFilter {
                frame_id: Some(frame_id),
                limit: Some(1),
                ..Default::default()
            })?
            .pop())
    }

    /// Story 17-8 AC2.6 — classify one frame for a participant caller under the
    /// shared predicate, in this adapter's boot, without loading its payload.
    /// `None` = no such frame.
    pub fn participant_access(
        &self,
        frame_id: [u8; 16],
        scope: ParticipantScope,
    ) -> Result<Option<ParticipantAccess>, AuditError> {
        let inner = self.inner.lock();
        let sql = format!(
            "WITH caller(pid, now_key) AS (VALUES (?1, ?2)) \
             SELECT t.spirit_pid, \
               t.boot_nonce = ?3 AND {kinds} AND (t.spirit_pid = caller.pid OR {RECIPIENT_PREDICATE}), \
               {participant} AND (t.spirit_pid = caller.pid OR NOT {HEADER_ONLY_PREDICATE}), \
               length(t.payload_redacted) \
             FROM transparency_log t, caller WHERE t.frame_id = ?4",
            kinds = disclosable_kinds_sql("t.kind"),
            participant = participant_predicate(),
        );
        inner
            .conn
            .query_row(
                &sql,
                rusqlite::params![
                    scope.pid as i64,
                    consent_order_key(scope.consent_now_ns),
                    inner.boot_nonce as i64,
                    &frame_id[..],
                ],
                |row| {
                    let owner_pid = row.get::<_, i64>(0)? as u32;
                    Ok(if !row.get::<_, bool>(1)? {
                        ParticipantAccess::OutOfScope { owner_pid }
                    } else if !row.get::<_, bool>(2)? {
                        ParticipantAccess::Withheld
                    } else {
                        ParticipantAccess::Payload {
                            payload_len: row.get::<_, i64>(3)? as u64,
                        }
                    })
                },
            )
            .optional()
            .map_err(AuditError::SqliteRead)
    }

    fn frame_query(
        filter: FrameFilter,
        projection: &str,
    ) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        // Story 17-8 AC2.5 — participant scope binds `(pid, now_key)` once in a
        // one-row CTE and filters in SQL, before any ORDER BY / LIMIT.
        let mut sql = match filter.participant {
            Some(scope) => {
                params.push(Box::new(scope.pid as i64));
                params.push(Box::new(consent_order_key(scope.consent_now_ns)));
                where_clauses.push(participant_predicate());
                format!(
                    "WITH caller(pid, now_key) AS (VALUES (?, ?)) \
                     SELECT {projection} FROM transparency_log t, caller"
                )
            }
            None => format!("SELECT {projection} FROM transparency_log t"),
        };

        if let Some(boot_nonce) = filter.boot_nonce {
            where_clauses.push("boot_nonce = ?".to_string());
            params.push(Box::new(boot_nonce as i64));
        }
        if let Some(pid) = filter.spirit_pid {
            where_clauses.push("spirit_pid = ?".to_string());
            params.push(Box::new(pid as i64));
        }
        if let Some(fid) = filter.frame_id {
            where_clauses.push("frame_id = ?".to_string());
            params.push(Box::new(fid.to_vec()));
        }
        if filter.disclosable_kinds_only {
            where_clauses.push(disclosable_kinds_sql("kind"));
        }
        if let Some(kind) = filter.kind {
            where_clauses.push("kind = ?".to_string());
            params.push(Box::new(kind as i64));
        }
        if let Some(intent) = filter.intent {
            where_clauses.push("intent = ?".to_string());
            params.push(Box::new(intent));
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
        if filter.order_by_insertion {
            if let Some(cursor_fid) = filter.cursor_frame_id {
                where_clauses.push(
                    "t.rowid > (SELECT rowid FROM transparency_log WHERE frame_id = ?)".to_string(),
                );
                params.push(Box::new(cursor_fid.to_vec()));
            }
        } else if let (Some(cursor_ts), Some(cursor_fid)) =
            (filter.cursor_timestamp_ns, filter.cursor_frame_id)
        {
            where_clauses.push("(timestamp_ns, frame_id) > (? , ?)".to_string());
            params.push(Box::new(cursor_ts as i64));
            params.push(Box::new(cursor_fid.to_vec()));
        }
        if let Some(cursor) = filter.cursor_insertion_id {
            where_clauses.push("t.rowid > ?".to_string());
            params.push(Box::new(cursor));
        }
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
        let order_key = match (filter.order_by_insertion, filter.descending) {
            (true, true) => "t.rowid DESC",
            (true, false) => "t.rowid ASC",
            (false, true) => "timestamp_ns DESC, frame_id DESC",
            (false, false) => "timestamp_ns, frame_id ASC",
        };
        sql.push_str(" ORDER BY ");
        sql.push_str(order_key);
        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit as i64));
        }
        (sql, params)
    }

    /// Read-side: look up a single frame by its primary key.
    /// Returns `None` if the frame_id is not found.
    pub fn query_frame_by_id(
        &self,
        frame_id: [u8; 16],
    ) -> Result<Option<TransparencyLogEntry>, AuditError> {
        let inner = self.inner.lock();
        let mut stmt = inner
            .conn
            .prepare(
                "SELECT rowid, frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                        boot_nonce, capability_token, kind, intent, correlation_id,
                        payload_redacted, origin FROM transparency_log
                 WHERE frame_id = ?1
                 LIMIT 1",
            )
            .map_err(AuditError::SqliteRead)?;

        let row = stmt
            .query_row(rusqlite::params![&frame_id[..]], |row| {
                let frame_id_blob: Vec<u8> = row.get(1)?;
                let mut fid = [0u8; 16];
                if frame_id_blob.len() == 16 {
                    fid.copy_from_slice(&frame_id_blob);
                }
                let cap_blob: Option<Vec<u8>> = row.get(7)?;
                let mut cap_token = None;
                if let Some(ref blob) = cap_blob {
                    if blob.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(blob);
                        cap_token = Some(arr);
                    }
                }
                Ok(TransparencyLogEntry {
                    insertion_id: row.get(0)?,
                    frame_id: fid,
                    timestamp_ns: row.get::<_, i64>(2)? as u64,
                    spirit_pid: row.get::<_, i64>(3)? as u32,
                    from_spirit_id: row.get(4)?,
                    to_spirit_id: row.get(5)?,
                    boot_nonce: row.get::<_, i64>(6)? as u64,
                    capability_token: cap_token,
                    kind: FrameKind::from_i64(row.get::<_, i64>(8)?).unwrap_or_else(|| {
                        let disc = row.get::<_, i64>(8).unwrap_or(-1);
                        eprintln!(
                            "TL query: unrecognized FrameKind discriminant ({disc}); \
                                 mapping to TaskAssign as best-effort fallback \
                                 (schema-migration or cross-version log inspection)"
                        );
                        FrameKind::TaskAssign
                    }),
                    intent: row.get(9)?,
                    correlation_id: row.get(10)?,
                    payload_redacted: row.get(11)?,
                    origin: match row.get::<_, i64>(12)? {
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
        let inner = self.inner.lock();

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
        let inner = self.inner.lock();
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
        let inner = self.inner.lock();
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
        let inner = self.inner.lock();
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
        let digests = [
            Some(entry.schema_content_hash.as_str()),
            entry.supersedes_hash.as_deref(),
        ];
        if !digests.iter().flatten().all(|t| Self::is_digest(t)) {
            return Err(AuditError::Serialization(
                "schema digests must be 64 hex digits".into(),
            ));
        }
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
            .map_err(|error| AuditError::Serialization(error.to_string()))?;
        let redacted = self.redaction.redact(&payload_bytes);
        let mut inner = self.inner.lock();
        let TransparencyLogInner {
            conn,
            next_frame_id_counter,
            boot_nonce,
            last_frame_id,
            ..
        } = &mut *inner;
        // Roll back even when an I2 write panic unwinds, without poisoning.
        let transaction = conn.transaction().map_err(AuditError::SqliteWriteFatal)?;
        transaction.execute(
            "INSERT INTO schema_lifecycle_registry
                (schema_id, version, effective_at_ns, supersedes_hash, ratified_by, recorded_at_ns, schema_content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                entry.schema_id, entry.version as i64, entry.effective_at_ns as i64,
                entry.supersedes_hash.as_deref(), entry.ratified_by,
                entry.recorded_at_ns as i64, entry.schema_content_hash,
            ],
        ).unwrap_or_else(|error| panic!("MAOS kernel panic — schema_lifecycle_registry write failed: {error}. I2."));
        let frame_id = Self::next_frame_id(next_frame_id_counter);
        let timestamp_ns = wall_clock_now_ns();
        transaction.execute(
            "INSERT INTO transparency_log
                (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce, capability_token,
                 kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                &frame_id[..], timestamp_ns as i64, 0i64, "", "", *boot_nonce as i64,
                Option::<&[u8]>::None, FrameKind::GovernanceEvent as i64,
                "governance:schema-lifecycle", &redacted[..], FrameOrigin::Kernel as i64,
            ],
        ).unwrap_or_else(|error| panic!("MAOS kernel panic — Transparency Log write failed: {error}. I2."));
        transaction.commit().map_err(AuditError::SqliteWriteFatal)?;
        *last_frame_id = frame_id;
        Ok(LogBeforeDeliver::new(()))
    }

    /// Story 9.3b (R10) — query the schema-lifecycle registry for a specific
    /// schema_id. Returns entries ordered by version ascending.
    pub fn query_schema_registry(
        &self,
        schema_id: &str,
    ) -> Result<Vec<maos_domain::governance::SchemaRegistryEntry>, AuditError> {
        let inner = self.inner.lock();
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
        let inner = self.inner.lock();
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
#[path = "erasure_scan_work.rs"]
mod erasure_scan_work;

#[cfg(test)]
#[path = "../../tests/support/erasure_races_20_11.rs"]
mod erasure_races_20_11;

#[cfg(test)]
#[path = "../../tests/support/transparency_log_unit.rs"]
mod tests;
