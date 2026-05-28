#![forbid(unsafe_code)]

//! Log-recall domain types per AC1 — `LogRecallFilter`, `LogRecallCursor`,
//! `LogRecallPage`, `LogRecallEntry`, `LogFetchResponse`, `LogRecallError`.
//!
//! These are the pure domain shape types consumed by `LogRecallPort` and
//! implemented by `LogRecallAdapter` in `maos-kernel-core`.

use thiserror::Error;

/// Filter for `LogRecallPort::recall`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogRecallFilter {
    /// Optional frame-kind filter.
    #[doc = "Construct via [`LogRecallFilter::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub kind: Option<FrameKindLabel>,

    /// Optional lower-bound timestamp (inclusive).
    #[doc = "Construct via [`LogRecallFilter::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub since_ns: Option<u64>,

    /// Optional upper-bound timestamp (inclusive).
    #[doc = "Construct via [`LogRecallFilter::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub until_ns: Option<u64>,

    /// Max entries to return. Clamped to MAX_LIMIT at construction.
    #[doc = "Construct via [`LogRecallFilter::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub limit: usize,

    /// Optional keyset-pagination cursor for the next page.
    #[doc = "Construct via [`LogRecallFilter::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub cursor: Option<LogRecallCursor>,

    /// Optional intent-class filter.
    #[doc = "Construct via [`LogRecallFilter::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub intent_filter: Option<String>,
}

/// Frame kind label for log-recall filtering (mirrors the audit-log FrameKind
/// discriminator without depending on maos-kernel-core).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FrameKindLabel {
    TaskAssign,
    TaskComplete,
    DecisionDispatch,
    EpistemicHalt,
    TelemetryEvent,
    ConsentRequest,
    Retract,
    CapabilityInvocation,
    SandboxBlock,
    InferenceCall,
    Decision,
    Distillate,
    BudgetWarning,
    BudgetExceeded,
    HotSwapAborted,
    TaskStalled,
    SilentFailureSuspect,
    SpiritRevoked,
    /// Story 5.5c — MCP tool invocation.
    McpInvocation,
    /// Story 5.5d — Spirit admitted to the kernel via registry.
    SpiritAdmitted,
    /// Story 5.5d — Registry yank propagated to the kernel.
    RegistryYank,
    /// Story 6.2 AC6 — FR52: a line of stdout/stderr captured from a
    /// CliWrapperSpirit's invoked CLI subprocess. Audit-style row; Spirits
    /// do not directly read this surface (the host kernel mediates it).
    CliSubprocessOutput,
    /// Story 6.4 — ADR-034 binding-v0.9: partial-consent rupture event.
    ConsentRupture,
    /// Story 6.4 — NFR-Scale-4: per-(provider, credential) rate-limit event.
    RateLimited,
    /// Story 6.5 — FR54: inbound message from external gateway.
    GatewayInbound,
    /// Story 6.5 — FR54: outbound message to external gateway.
    GatewayOutbound,
}

impl LogRecallFilter {
    /// Maximum entries per recall page.
    pub const MAX_LIMIT: usize = 1024;

    /// Construct a validated filter. Limits are silently clamped to MAX_LIMIT;
    /// v0.5+ promotes the clamp to a typed error once corpus-test ergonomics settle.
    pub fn new(
        kind: Option<FrameKindLabel>,
        since_ns: Option<u64>,
        until_ns: Option<u64>,
        limit: usize,
        cursor: Option<LogRecallCursor>,
        intent_filter: Option<String>,
    ) -> Self {
        Self {
            kind,
            since_ns,
            until_ns,
            limit: limit.min(Self::MAX_LIMIT),
            cursor,
            intent_filter,
        }
    }
}

impl Default for LogRecallFilter {
    fn default() -> Self {
        Self {
            kind: None,
            since_ns: None,
            until_ns: None,
            limit: Self::MAX_LIMIT,
            cursor: None,
            intent_filter: None,
        }
    }
}

/// Keyset-pagination cursor.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogRecallCursor {
    #[doc = "Construct via [`LogRecallCursor::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub last_timestamp_ns: u64,
    #[doc = "Construct via [`LogRecallCursor::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub last_frame_id: [u8; 16],
}

impl LogRecallCursor {
    pub fn new(last_timestamp_ns: u64, last_frame_id: [u8; 16]) -> Self {
        Self {
            last_timestamp_ns,
            last_frame_id,
        }
    }
}

/// A page of recall results.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogRecallPage {
    #[doc = "Construct via [`LogRecallPage::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub entries: Vec<LogRecallEntry>,
    #[doc = "Construct via [`LogRecallPage::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub next_cursor: Option<LogRecallCursor>,
}

impl LogRecallPage {
    pub fn new(entries: Vec<LogRecallEntry>, next_cursor: Option<LogRecallCursor>) -> Self {
        Self {
            entries,
            next_cursor,
        }
    }
}

/// A single recall entry — intentionally OMITS the raw payload; consumers
/// MUST call `fetch(frame_id)` for the payload (lazy-load to honor A2A
/// consent re-check at the moment of payload disclosure).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogRecallEntry {
    #[doc = "Construct via [`LogRecallEntry::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub frame_id: [u8; 16],
    #[doc = "Construct via [`LogRecallEntry::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub timestamp_ns: u64,
    #[doc = "Construct via [`LogRecallEntry::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub kind: FrameKindLabel,
    #[doc = "Construct via [`LogRecallEntry::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub intent: String,
    #[doc = "Construct via [`LogRecallEntry::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub peer_spirit_pid: u32,
    #[doc = "Construct via [`LogRecallEntry::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub payload_available: bool,
}

impl LogRecallEntry {
    pub fn new(
        frame_id: [u8; 16],
        timestamp_ns: u64,
        kind: FrameKindLabel,
        intent: String,
        peer_spirit_pid: u32,
        payload_available: bool,
    ) -> Self {
        Self {
            frame_id,
            timestamp_ns,
            kind,
            intent,
            peer_spirit_pid,
            payload_available,
        }
    }
}

/// Response from `LogRecallPort::fetch` — the full frame including payload.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogFetchResponse {
    #[doc = "Construct via [`LogFetchResponse::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub frame_id: [u8; 16],
    #[doc = "Construct via [`LogFetchResponse::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub timestamp_ns: u64,
    #[doc = "Construct via [`LogFetchResponse::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub kind: FrameKindLabel,
    #[doc = "Construct via [`LogFetchResponse::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub intent: String,
    #[doc = "Construct via [`LogFetchResponse::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub payload_redacted: Vec<u8>,
    #[doc = "Construct via [`LogFetchResponse::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub capability_token: Option<[u8; 32]>,
    #[doc = "Construct via [`LogFetchResponse::new`] to enforce validation; struct literals bypass cursor-ordering / pid-range / limit-cap checks."]
    pub origin: crate::invariants::i3::FrameOrigin,
}

impl LogFetchResponse {
    pub fn new(
        frame_id: [u8; 16],
        timestamp_ns: u64,
        kind: FrameKindLabel,
        intent: String,
        payload_redacted: Vec<u8>,
        capability_token: Option<[u8; 32]>,
        origin: crate::invariants::i3::FrameOrigin,
    ) -> Self {
        Self {
            frame_id,
            timestamp_ns,
            kind,
            intent,
            payload_redacted,
            capability_token,
            origin,
        }
    }
}

/// Typed error for log-recall operations.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum LogRecallError {
    /// Cross-Spirit fetch — the requesting Spirit is not the emitter.
    #[error("E_SCOPE_VIOLATION — frame {frame_id:?} owned by pid {owner_pid}, requested by pid {requested_pid}")]
    ScopeViolation {
        frame_id: [u8; 16],
        requested_pid: u32,
        owner_pid: u32,
    },
    /// Requested frame does not exist in the Transparency Log.
    #[error("frame {frame_id:?} not found")]
    FrameNotFound { frame_id: [u8; 16] },
    /// Storage backend error.
    #[error("storage error: {0}")]
    Storage(String),
    /// Malformed cursor or cursor pointing to a non-existent row.
    #[error("invalid cursor: {0}")]
    InvalidCursor(String),
    /// Hard limit exceeded (v0.5+ promotion from silent clamp).
    #[error("limit exceeded: requested {requested}, max {max}")]
    LimitExceeded { requested: usize, max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_new_clamps_limit() {
        let filter = LogRecallFilter::new(None, None, None, usize::MAX, None, None);
        assert_eq!(filter.limit, LogRecallFilter::MAX_LIMIT);
    }

    #[test]
    fn filter_default_sets_max_limit() {
        let filter = LogRecallFilter::default();
        assert_eq!(filter.limit, LogRecallFilter::MAX_LIMIT);
        assert!(filter.kind.is_none());
        assert!(filter.cursor.is_none());
    }

    #[test]
    fn log_recall_page_serde_round_trip() {
        let page = LogRecallPage::new(
            vec![LogRecallEntry::new(
                [1u8; 16],
                100,
                FrameKindLabel::TaskAssign,
                "delegate".into(),
                7,
                true,
            )],
            Some(LogRecallCursor::new(100, [1u8; 16])),
        );
        let json = serde_json::to_string(&page).unwrap();
        let back: LogRecallPage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].intent, "delegate");
        assert!(back.next_cursor.is_some());
    }

    #[test]
    fn log_recall_error_scope_violation_carries_fields() {
        let err = LogRecallError::ScopeViolation {
            frame_id: [0xAA; 16],
            requested_pid: 20,
            owner_pid: 10,
        };
        match err {
            LogRecallError::ScopeViolation {
                frame_id,
                requested_pid,
                owner_pid,
            } => {
                assert_eq!(frame_id, [0xAA; 16]);
                assert_eq!(requested_pid, 20);
                assert_eq!(owner_pid, 10);
            }
            _ => panic!("expected ScopeViolation"),
        }
        let display = format!("{err}");
        assert!(display.contains("SCOPE_VIOLATION"));
    }

    #[test]
    fn log_recall_error_distinguishes_variants() {
        assert_ne!(
            LogRecallError::FrameNotFound {
                frame_id: [0u8; 16]
            },
            LogRecallError::InvalidCursor("".into()),
        );
    }

    #[test]
    fn log_recall_cursor_round_trip() {
        let cursor = LogRecallCursor::new(500, [0xBB; 16]);
        let json = serde_json::to_string(&cursor).unwrap();
        let back: LogRecallCursor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_timestamp_ns, 500);
        assert_eq!(back.last_frame_id, [0xBB; 16]);
    }

    #[test]
    fn log_fetch_response_serde_round_trip() {
        let resp = LogFetchResponse::new(
            [0xCC; 16],
            200,
            FrameKindLabel::Distillate,
            "distillate.write".into(),
            b"payload".to_vec(),
            Some([0xDD; 32]),
            crate::invariants::i3::FrameOrigin::SpiritDraftedHumanApproved,
        );
        let json = serde_json::to_string(&resp).unwrap();
        let back: LogFetchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frame_id, [0xCC; 16]);
        assert_eq!(back.intent, "distillate.write");
        assert_eq!(back.capability_token, Some([0xDD; 32]));
    }
}
