#![forbid(unsafe_code)]

//! Log-recall adapter — implements `LogRecallPort` over the Transparency Log.
//!
//! Participant-scoped (Story 17-8), boot-incarnation-bound metadata recall and
//! lazy payload fetch: a Spirit recalls the frames it sent or was recorded as
//! receiving in this boot, each received frame still under its granter's
//! persisted consent. Cross-wall reads stay emitter-scoped (ADR-058 D2).

use std::sync::Arc;

#[cfg(feature = "spirit_test")]
use maos_spirit_sdk::spirit_test::{AttemptResult, IsolationHookPoint, ObservationResult};
#[cfg(feature = "spirit_test")]
use parking_lot::Mutex;

use maos_domain::invariants::i1::TokenId;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{
    CrossWallRecallRefusal, EmitterScopeReason, FrameKindLabel as DomainFrameKindLabel,
    LogFetchResponse, LogRecallCursor, LogRecallEntry, LogRecallError, LogRecallFilter,
    LogRecallPage, RecallScope,
};
use maos_domain::ports::{
    CrossWallLogReadPort, CrossWallRecallConsentDecision, CrossWallRecallConsentError,
    CrossWallRecallConsentPort, LogRecallPort,
};
use maos_domain::team::TeamId;

use super::transparency_log::{
    capability_token_column, FrameFilter, FrameKind, ParticipantAccess, ParticipantScope,
    TransparencyLogAdapter, TransparencyLogMetadata, SPIRIT_DISCLOSABLE_RAW_KINDS,
};

/// Stable intent string constant for `CapabilityInvocation` audit row.
pub const LOG_RECALL_INTENT: &str = "log.recall";
/// Canonical A2A intent used for signed directional cross-team grants.
pub const CROSS_WALL_RECALL_CONSENT_INTENT: &str = "log:recall";
/// Audit intent for a consent-governed cross-wall disclosure or refusal.
pub const LOG_CROSS_WALL_RECALL_INTENT: &str = "log.recall.cross-wall";

/// Stable intent string constant for `CapabilityInvocation` audit row.
pub const LOG_FETCH_INTENT: &str = "log.fetch";

/// Only bus frames and MCP invocations are Spirit-disclosable. Inspect the raw
/// SQLite discriminant, never an audit reader's decoded fallback. Same list as
/// the SQL predicate ([`SPIRIT_DISCLOSABLE_RAW_KINDS`]).
pub fn is_spirit_disclosable(raw_kind: i64) -> bool {
    SPIRIT_DISCLOSABLE_RAW_KINDS.contains(&raw_kind)
}

/// Story 17-8 AC2.5 / AC6.1 — which rows a page may be drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageScope {
    /// Local recall: this boot only; rows the caller sent or was recorded as
    /// receiving, received rows judged against consent at `consent_now_ns`.
    Participant {
        boot_nonce: u64,
        consent_now_ns: u64,
    },
    /// Rows the caller emitted, any boot, no side table read (a foreign
    /// artifact may predate them). The page carries the emitter marker.
    Emitter(EmitterScopeReason),
}

impl PageScope {
    fn marker(self) -> RecallScope {
        match self {
            Self::Participant { .. } => RecallScope::Participant,
            Self::Emitter(reason) => RecallScope::Emitter { reason },
        }
    }
}

/// Story 17-8 AC3.2 — the consent clock seam. Production reads
/// [`maos_domain::frame::consent_wall_now_ns`]; tests pin and advance it.
pub type ConsentClock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// A bounded fetch refuses an oversized stored payload before loading its blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedFetch {
    Fetched(LogFetchResponse),
    PayloadTooLarge { payload_len: u64 },
}

/// Log-recall adapter — stateless composer over `Arc<TransparencyLogAdapter>`.
///
/// Does NOT require `#[i9_exempt]` — holds only an `Arc` reference to
/// an existing I9-sanctioned holder (same shape as `SelfTelemetryAggregator`).
pub struct LogRecallAdapter {
    transparency_log: Arc<TransparencyLogAdapter>,
    consent_clock: ConsentClock,
    cross_wall_consent: Option<Arc<dyn CrossWallRecallConsentPort>>,
    cross_wall_read: Option<Arc<dyn CrossWallLogReadPort>>,
    /// Story 4.4 — cross-Spirit isolation hook (Story 4.5 corpus).
    /// Feature-gated so production builds carry zero runtime cost.
    #[cfg(feature = "spirit_test")]
    isolation_hook: Option<Arc<Mutex<dyn IsolationHookPoint + Send>>>,
}

impl LogRecallAdapter {
    /// Construct a new adapter wrapping the Transparency Log.
    pub fn new(transparency_log: Arc<TransparencyLogAdapter>) -> Self {
        Self {
            transparency_log,
            consent_clock: Arc::new(maos_domain::frame::consent_wall_now_ns),
            cross_wall_consent: None,
            cross_wall_read: None,
            #[cfg(feature = "spirit_test")]
            isolation_hook: None,
        }
    }

    /// Story 17-8 AC3.2 — replace the consent clock (tests pin and advance it).
    pub fn with_consent_clock(mut self, clock: ConsentClock) -> Self {
        self.consent_clock = clock;
        self
    }

    /// Attach the directional cross-wall consent decision seam. The builder is
    /// unconditional; `new` remains one-argument for all existing consumers.
    pub fn with_cross_wall_consent(mut self, consent: Arc<dyn CrossWallRecallConsentPort>) -> Self {
        self.cross_wall_consent = Some(consent);
        self
    }

    /// Attach the dependency-inverted remote audit-artifact reader.
    pub fn with_cross_wall_read(mut self, read: Arc<dyn CrossWallLogReadPort>) -> Self {
        self.cross_wall_read = Some(read);
        self
    }

    fn journal_cross_wall_recall(
        &self,
        spirit_pid: u32,
        remote_team: &TeamId,
        outcome: &str,
        refusal: Option<&CrossWallRecallRefusal>,
    ) {
        let outcome = refusal
            .map(CrossWallRecallRefusal::outcome)
            .unwrap_or(outcome);
        let consent_granted = refusal.is_none();
        let crossed = outcome == "disclosed";
        let payload = serde_json::json!({
            "remote_team": remote_team.as_str(),
            "outcome": outcome,
            "consent_grant": consent_granted.then_some(CROSS_WALL_RECALL_CONSENT_INTENT),
            "crossed_team_boundary": crossed,
            "refusal": refusal.map(|reason| serde_json::json!({
                "code": reason.code(),
                "detail": reason.to_string(),
            })),
        });
        self.transparency_log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            LOG_CROSS_WALL_RECALL_INTENT,
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );
    }

    /// Story 4.4 — attach an isolation hook for the `spirit_test` feature.
    /// Only available when the `spirit_test` feature is enabled.
    #[cfg(feature = "spirit_test")]
    pub fn with_isolation_hook(mut self, hook: Arc<Mutex<dyn IsolationHookPoint + Send>>) -> Self {
        self.isolation_hook = Some(hook);
        self
    }

    #[cfg(feature = "spirit_test")]
    fn fire_isolation_hooks(&self, case_id: &str, attempt_ok: bool) {
        if let Some(hook) = &self.isolation_hook {
            let mut h = hook.lock();
            let _ = h.before_spirit_a_attempt(case_id);
            let attempt = AttemptResult {
                hooks_fired_during_attempt: vec![case_id.into()],
                frames_emitted: if attempt_ok { 1 } else { 0 },
            };
            let _ = h.after_spirit_a_attempt(case_id, &attempt);
            let _ = h.before_spirit_b_observe(case_id);
            let observation = ObservationResult {
                hooks_fired_during_observation: vec![case_id.into()],
                frames_emitted: 0,
                leaked_bytes: None,
            };
            let _ = h.after_spirit_b_observe(case_id, &observation);
        }
    }

    /// Map kernel-side `FrameKind` → domain `FrameKindLabel`.
    fn to_domain_kind(kind: FrameKind) -> DomainFrameKindLabel {
        match kind {
            FrameKind::TaskAssign => DomainFrameKindLabel::TaskAssign,
            FrameKind::TaskComplete => DomainFrameKindLabel::TaskComplete,
            FrameKind::DecisionDispatch => DomainFrameKindLabel::DecisionDispatch,
            FrameKind::EpistemicHalt => DomainFrameKindLabel::EpistemicHalt,
            FrameKind::TelemetryEvent => DomainFrameKindLabel::TelemetryEvent,
            FrameKind::ConsentRequest => DomainFrameKindLabel::ConsentRequest,
            FrameKind::Retract => DomainFrameKindLabel::Retract,
            FrameKind::CapabilityInvocation => DomainFrameKindLabel::CapabilityInvocation,
            FrameKind::SandboxBlock => DomainFrameKindLabel::SandboxBlock,
            FrameKind::InferenceCall => DomainFrameKindLabel::InferenceCall,
            FrameKind::Decision => DomainFrameKindLabel::Decision,
            FrameKind::Distillate => DomainFrameKindLabel::Distillate,
            FrameKind::BudgetWarning => DomainFrameKindLabel::BudgetWarning,
            FrameKind::BudgetExceeded => DomainFrameKindLabel::BudgetExceeded,
            FrameKind::HotSwapAborted => DomainFrameKindLabel::HotSwapAborted,
            FrameKind::TaskStalled => DomainFrameKindLabel::TaskStalled,
            FrameKind::SilentFailureSuspect => DomainFrameKindLabel::SilentFailureSuspect,
            FrameKind::SpiritRevoked => DomainFrameKindLabel::SpiritRevoked,
            FrameKind::McpInvocation => DomainFrameKindLabel::McpInvocation,
            FrameKind::SpiritAdmitted => DomainFrameKindLabel::SpiritAdmitted,
            FrameKind::RegistryYank => DomainFrameKindLabel::RegistryYank,
            FrameKind::CliSubprocessOutput => DomainFrameKindLabel::CliSubprocessOutput,
            FrameKind::ConsentRupture => DomainFrameKindLabel::ConsentRupture,
            FrameKind::RateLimited => DomainFrameKindLabel::RateLimited,
            FrameKind::GatewayInbound => DomainFrameKindLabel::GatewayInbound,
            FrameKind::GatewayOutbound => DomainFrameKindLabel::GatewayOutbound,
            FrameKind::SpiritImported => DomainFrameKindLabel::SpiritImported,
            FrameKind::CliWrapperShapeMismatch => DomainFrameKindLabel::CliWrapperShapeMismatch,
            FrameKind::GovernanceEvent => DomainFrameKindLabel::GovernanceEvent,
            FrameKind::CostAttribution => DomainFrameKindLabel::CostAttribution,
        }
    }

    /// Map domain `FrameKindLabel` → kernel-side `FrameKind`.
    ///
    /// An unsupported label has no matches; never silently substitute a
    /// different kind or widen the query to all kinds.
    fn to_kernel_kind(label: &DomainFrameKindLabel) -> Option<FrameKind> {
        match label {
            DomainFrameKindLabel::TaskAssign => Some(FrameKind::TaskAssign),
            DomainFrameKindLabel::TaskComplete => Some(FrameKind::TaskComplete),
            DomainFrameKindLabel::DecisionDispatch => Some(FrameKind::DecisionDispatch),
            DomainFrameKindLabel::EpistemicHalt => Some(FrameKind::EpistemicHalt),
            DomainFrameKindLabel::TelemetryEvent => Some(FrameKind::TelemetryEvent),
            DomainFrameKindLabel::ConsentRequest => Some(FrameKind::ConsentRequest),
            DomainFrameKindLabel::Retract => Some(FrameKind::Retract),
            DomainFrameKindLabel::CapabilityInvocation => Some(FrameKind::CapabilityInvocation),
            DomainFrameKindLabel::SandboxBlock => Some(FrameKind::SandboxBlock),
            DomainFrameKindLabel::InferenceCall => Some(FrameKind::InferenceCall),
            DomainFrameKindLabel::Decision => Some(FrameKind::Decision),
            DomainFrameKindLabel::Distillate => Some(FrameKind::Distillate),
            DomainFrameKindLabel::BudgetWarning => Some(FrameKind::BudgetWarning),
            DomainFrameKindLabel::BudgetExceeded => Some(FrameKind::BudgetExceeded),
            DomainFrameKindLabel::HotSwapAborted => Some(FrameKind::HotSwapAborted),
            DomainFrameKindLabel::TaskStalled => Some(FrameKind::TaskStalled),
            DomainFrameKindLabel::SilentFailureSuspect => Some(FrameKind::SilentFailureSuspect),
            DomainFrameKindLabel::SpiritRevoked => Some(FrameKind::SpiritRevoked),
            DomainFrameKindLabel::McpInvocation => Some(FrameKind::McpInvocation),
            DomainFrameKindLabel::SpiritAdmitted => Some(FrameKind::SpiritAdmitted),
            DomainFrameKindLabel::RegistryYank => Some(FrameKind::RegistryYank),
            DomainFrameKindLabel::CliSubprocessOutput => Some(FrameKind::CliSubprocessOutput),
            DomainFrameKindLabel::ConsentRupture => Some(FrameKind::ConsentRupture),
            DomainFrameKindLabel::RateLimited => Some(FrameKind::RateLimited),
            DomainFrameKindLabel::GatewayInbound => Some(FrameKind::GatewayInbound),
            DomainFrameKindLabel::GatewayOutbound => Some(FrameKind::GatewayOutbound),
            DomainFrameKindLabel::SpiritImported => Some(FrameKind::SpiritImported),
            DomainFrameKindLabel::CliWrapperShapeMismatch => {
                Some(FrameKind::CliWrapperShapeMismatch)
            }
            DomainFrameKindLabel::GovernanceEvent => Some(FrameKind::GovernanceEvent),
            DomainFrameKindLabel::CostAttribution => Some(FrameKind::CostAttribution),
            _ => None,
        }
    }

    fn build_entry(tl_entry: TransparencyLogMetadata) -> Result<LogRecallEntry, LogRecallError> {
        if !is_spirit_disclosable(tl_entry.raw_kind) {
            return Err(LogRecallError::Storage(
                "non-disclosable recall metadata".into(),
            ));
        }
        let kind = FrameKind::from_i64(tl_entry.raw_kind)
            .ok_or_else(|| LogRecallError::Storage("unrecognized recall kind".into()))?;
        Ok(LogRecallEntry::new(
            tl_entry.frame_id,
            tl_entry.timestamp_ns,
            Self::to_domain_kind(kind),
            tl_entry.intent,
            tl_entry.spirit_pid,
            tl_entry.payload_len != 0,
        ))
    }

    fn validate_filter(filter: &LogRecallFilter) -> Result<(), LogRecallError> {
        for timestamp in [
            filter.since_ns,
            filter.until_ns,
            filter
                .cursor
                .as_ref()
                .map(|cursor| cursor.last_timestamp_ns),
        ]
        .into_iter()
        .flatten()
        {
            if timestamp > i64::MAX as u64 {
                return Err(LogRecallError::InvalidCursor(
                    "timestamp exceeds SQLite's representable range".into(),
                ));
            }
        }
        Ok(())
    }

    /// Query one page in `scope` without emitting a local audit row; every
    /// predicate runs in SQL before `LIMIT`.
    ///
    /// Composition-root remote readers reuse this mapping (emitter scope) over a
    /// connection that SQLite has already opened read-only.
    pub fn query_page(
        transparency_log: &TransparencyLogAdapter,
        spirit_pid: u32,
        scope: PageScope,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
        Self::validate_filter(&filter)?;
        let effective_limit = filter.limit.min(LogRecallFilter::MAX_LIMIT);
        if effective_limit == 0 {
            return Ok(LogRecallPage::new(Vec::new(), None, scope.marker()));
        }
        let kind = match filter.kind.as_ref().map(Self::to_kernel_kind) {
            Some(None) => return Ok(LogRecallPage::new(Vec::new(), None, scope.marker())),
            Some(Some(kind)) => Some(kind),
            None => None,
        };
        let (emitter, boot_nonce, participant) = match scope {
            PageScope::Participant {
                boot_nonce,
                consent_now_ns,
            } => (
                None,
                Some(boot_nonce),
                Some(ParticipantScope {
                    pid: spirit_pid,
                    consent_now_ns,
                }),
            ),
            PageScope::Emitter(_) => (Some(spirit_pid), None, None),
        };
        let kernel_filter = FrameFilter {
            spirit_pid: emitter,
            boot_nonce,
            participant,
            disclosable_kinds_only: true,
            intent: filter.intent_filter,
            kind,
            since_ns: filter.since_ns,
            until_ns: filter.until_ns,
            limit: Some(effective_limit.saturating_add(1)),
            cursor_timestamp_ns: filter
                .cursor
                .as_ref()
                .map(|cursor| cursor.last_timestamp_ns),
            cursor_frame_id: filter.cursor.as_ref().map(|cursor| cursor.last_frame_id),
            order_by_insertion: filter.order_by_insertion,
            ..Default::default()
        };
        let mut selected = transparency_log
            .query_frame_metadata(kernel_filter)
            .map_err(|error| LogRecallError::Storage(error.to_string()))?;
        let next_cursor = if selected.len() > effective_limit {
            selected.truncate(effective_limit);
            selected
                .last()
                .map(|last| LogRecallCursor::new(last.timestamp_ns, last.frame_id))
        } else {
            None
        };
        let entries = selected
            .into_iter()
            .map(Self::build_entry)
            .collect::<Result<_, _>>()?;
        Ok(LogRecallPage::new(entries, next_cursor, scope.marker()))
    }

    fn participant(&self, spirit_pid: u32) -> ParticipantScope {
        ParticipantScope {
            pid: spirit_pid,
            consent_now_ns: (self.consent_clock)(),
        }
    }

    /// Convert kernel origin → domain origin.
    fn to_domain_origin(origin: FrameOrigin) -> maos_domain::invariants::i3::FrameOrigin {
        origin
    }

    /// Recall with the minted capability stamped on its single invocation row.
    pub fn recall_attributed(
        &self,
        spirit_pid: u32,
        filter: LogRecallFilter,
        token: &TokenId,
    ) -> Result<LogRecallPage, LogRecallError> {
        self.recall_inner(spirit_pid, filter, Some(token))
    }

    /// Fetch with the minted capability stamped on its single invocation row.
    pub fn fetch_attributed(
        &self,
        spirit_pid: u32,
        frame_id: [u8; 16],
        token: &TokenId,
    ) -> Result<LogFetchResponse, LogRecallError> {
        match self.fetch_inner(spirit_pid, frame_id, Some(token), u64::MAX)? {
            BoundedFetch::Fetched(response) => Ok(response),
            BoundedFetch::PayloadTooLarge { .. } => unreachable!("unbounded fetch"),
        }
    }

    /// Enforce the stored-payload byte bound before loading its blob. Scope,
    /// incarnation and disclosure checks precede size refusal. The transport
    /// must still check its exact encoded reply size after a successful fetch.
    pub fn fetch_attributed_bounded(
        &self,
        spirit_pid: u32,
        frame_id: [u8; 16],
        token: &TokenId,
        max_payload_len: u64,
    ) -> Result<BoundedFetch, LogRecallError> {
        self.fetch_inner(spirit_pid, frame_id, Some(token), max_payload_len)
    }

    fn recall_inner(
        &self,
        spirit_pid: u32,
        filter: LogRecallFilter,
        token: Option<&TokenId>,
    ) -> Result<LogRecallPage, LogRecallError> {
        Self::validate_filter(&filter)?;
        let audit_payload = serde_json::json!({
            "limit": filter.limit,
            "cursor_present": filter.cursor.is_some(),
        });
        self.transparency_log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            token.map(capability_token_column).as_ref(),
            LOG_RECALL_INTENT,
            audit_payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&format!("log.recall:{spirit_pid}"), true);

        Self::query_page(
            &self.transparency_log,
            spirit_pid,
            PageScope::Participant {
                boot_nonce: self.transparency_log.boot_nonce(),
                consent_now_ns: (self.consent_clock)(),
            },
            filter,
        )
    }

    fn fetch_inner(
        &self,
        spirit_pid: u32,
        frame_id: [u8; 16],
        token: Option<&TokenId>,
        max_payload_len: u64,
    ) -> Result<BoundedFetch, LogRecallError> {
        let frame_id_hex: String = frame_id.iter().map(|b| format!("{b:02x}")).collect();
        let audit_payload = serde_json::json!({ "frame_id_hex": frame_id_hex });
        self.transparency_log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            token.map(capability_token_column).as_ref(),
            LOG_FETCH_INTENT,
            audit_payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );
        #[cfg(feature = "spirit_test")]
        {
            let hex_prefix: String = frame_id
                .iter()
                .take(4)
                .map(|b| format!("{b:02x}"))
                .collect();
            self.fire_isolation_hooks(&format!("log.fetch:{spirit_pid}:{hex_prefix}"), true);
        }

        // Story 17-8 AC2.6 — not found, then scope (foreign boot, undisclosable
        // kind, non-participant: indistinguishable), then the granter's consent,
        // and only then the byte bound — a withheld or header-only frame never
        // reports `PayloadTooLarge` and its payload is never loaded.
        let scope = self.participant(spirit_pid);
        let access = self
            .transparency_log
            .participant_access(frame_id, scope)
            .map_err(|error| LogRecallError::Storage(error.to_string()))?
            .ok_or(LogRecallError::FrameNotFound { frame_id })?;
        let payload_len = match access {
            ParticipantAccess::OutOfScope { owner_pid } => {
                return Err(LogRecallError::ScopeViolation {
                    frame_id,
                    requested_pid: spirit_pid,
                    owner_pid,
                })
            }
            ParticipantAccess::Withheld => {
                return Err(LogRecallError::ConsentWithheld { frame_id })
            }
            ParticipantAccess::Payload { payload_len } => payload_len,
        };
        if payload_len > max_payload_len {
            return Ok(BoundedFetch::PayloadTooLarge { payload_len });
        }
        // The load re-applies the participant predicate at the same `now`, so a
        // rupture journaled since the classification still withholds it.
        let entry = self
            .transparency_log
            .query_frames(FrameFilter {
                frame_id: Some(frame_id),
                boot_nonce: Some(self.transparency_log.boot_nonce()),
                participant: Some(scope),
                disclosable_kinds_only: true,
                limit: Some(1),
                ..Default::default()
            })
            .map_err(|error| LogRecallError::Storage(error.to_string()))?
            .pop()
            .ok_or(LogRecallError::ConsentWithheld { frame_id })?;
        Ok(BoundedFetch::Fetched(LogFetchResponse::new(
            entry.frame_id,
            entry.timestamp_ns,
            Self::to_domain_kind(entry.kind),
            entry.intent,
            entry.payload_redacted,
            entry.capability_token,
            Self::to_domain_origin(entry.origin),
        )))
    }
}

impl LogRecallPort for LogRecallAdapter {
    fn recall(
        &self,
        spirit_pid: u32,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
        self.recall_inner(spirit_pid, filter, None)
    }

    fn recall_cross_wall(
        &self,
        spirit_pid: u32,
        team: &TeamId,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
        Self::validate_filter(&filter)?;
        let reason = match self.cross_wall_consent.as_ref() {
            None => Some(CrossWallRecallRefusal::NoConsentProvider),
            Some(consent) => match consent.decide(team, CROSS_WALL_RECALL_CONSENT_INTENT) {
                Ok(CrossWallRecallConsentDecision::Granted) => None,
                Ok(CrossWallRecallConsentDecision::NoGrant) => {
                    Some(CrossWallRecallRefusal::NoGrant)
                }
                Ok(CrossWallRecallConsentDecision::WrongDirection) => {
                    Some(CrossWallRecallRefusal::WrongDirection)
                }
                Err(CrossWallRecallConsentError::Stale { reason }) => {
                    Some(CrossWallRecallRefusal::ConsentStateStale(reason))
                }
                Err(CrossWallRecallConsentError::StateUnavailable { reason }) => {
                    Some(CrossWallRecallRefusal::ConsentStateUnavailable(reason))
                }
            },
        };
        if let Some(reason) = reason {
            self.journal_cross_wall_recall(spirit_pid, team, "refused", Some(&reason));
            return Err(LogRecallError::ECrossWallRecallDenied {
                team: team.clone(),
                reason,
            });
        }
        let Some(read) = self.cross_wall_read.as_ref() else {
            let reason = CrossWallRecallRefusal::ReadPortUnavailable;
            self.journal_cross_wall_recall(spirit_pid, team, "refused", Some(&reason));
            return Err(LogRecallError::ECrossWallRecallDenied {
                team: team.clone(),
                reason,
            });
        };
        // FR4: record the consented cross-wall disclosure intent BEFORE data
        // movement, then journal the truthful outcome once the read resolves.
        // A single pre-movement "disclosed" would be a false disclosure on a
        // read that later fails (the D-6 truthfulness property this enforces).
        self.journal_cross_wall_recall(spirit_pid, team, "disclosing", None);
        match read.read_remote(spirit_pid, team, filter) {
            Ok(page) => {
                self.journal_cross_wall_recall(spirit_pid, team, "disclosed", None);
                Ok(page)
            }
            Err(error) => {
                self.journal_cross_wall_recall(spirit_pid, team, "failed", None);
                Err(error)
            }
        }
    }

    fn fetch(
        &self,
        spirit_pid: u32,
        frame_id: [u8; 16],
    ) -> Result<LogFetchResponse, LogRecallError> {
        match self.fetch_inner(spirit_pid, frame_id, None, u64::MAX)? {
            BoundedFetch::Fetched(response) => Ok(response),
            BoundedFetch::PayloadTooLarge { .. } => unreachable!("unbounded fetch"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::transparency_log::TransparencyLogEntry;
    use super::*;
    use maos_domain::ports::{
        CrossWallRecallConsentError, CrossWallRecallConsentPort, LogRecallPort,
    };
    use maos_domain::team::TeamId;

    fn make_adapter(nonce: u64) -> LogRecallAdapter {
        LogRecallAdapter::new(Arc::new(TransparencyLogAdapter::open_in_memory(nonce)))
    }

    fn seed_frames(tl: &Arc<TransparencyLogAdapter>, pid: u32, count: usize) {
        for i in 0..count {
            let payload = format!("payload-{pid}-{i}");
            let _token = tl.insert_frame_event(
                FrameKind::TaskAssign,
                pid,
                None,
                "delegate",
                payload.as_bytes(),
                FrameOrigin::HumanAuthored,
            );
        }
    }

    fn cross_wall_audit_rows(tl: &TransparencyLogAdapter) -> Vec<TransparencyLogEntry> {
        tl.query_frames(FrameFilter {
            kind: Some(FrameKind::CapabilityInvocation),
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .filter(|entry| entry.intent == LOG_CROSS_WALL_RECALL_INTENT)
        .collect()
    }

    #[test]
    fn recall_emitter_scope_only_returns_own_frames() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD401));
        seed_frames(&tl, 10, 5);
        seed_frames(&tl, 20, 5);
        seed_frames(&tl, 30, 3);

        let adapter = LogRecallAdapter::new(tl);
        let page = adapter.recall(10, LogRecallFilter::default()).unwrap();

        assert_eq!(page.entries.len(), 5);
        assert!(page.entries.iter().all(|e| e.peer_spirit_pid == 10));
    }

    #[test]
    fn cursor_pagination_three_page_walk() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD402));
        seed_frames(&tl, 10, 5);

        let adapter = LogRecallAdapter::new(tl);

        // Page 1: limit=2
        let page1 = adapter
            .recall(10, LogRecallFilter::new(None, None, None, 2, None, None))
            .unwrap();
        assert_eq!(page1.entries.len(), 2);
        assert!(page1.next_cursor.is_some());

        // Page 2: continue with cursor
        let page2 = adapter
            .recall(
                10,
                LogRecallFilter::new(None, None, None, 2, page1.next_cursor, None),
            )
            .unwrap();
        assert_eq!(page2.entries.len(), 2);
        assert!(page2.next_cursor.is_some());

        // Page 3: final page (1 entry + no cursor)
        let page3 = adapter
            .recall(
                10,
                LogRecallFilter::new(None, None, None, 2, page2.next_cursor, None),
            )
            .unwrap();
        assert_eq!(page3.entries.len(), 1);
        assert!(page3.next_cursor.is_none());
    }

    #[test]
    fn fetch_returns_owned_frame() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD403));
        seed_frames(&tl, 10, 1);
        let page = LogRecallAdapter::new(Arc::clone(&tl))
            .recall(10, LogRecallFilter::default())
            .unwrap();
        let frame_id = page.entries[0].frame_id;

        let adapter = LogRecallAdapter::new(tl);
        let resp = adapter.fetch(10, frame_id).unwrap();
        assert_eq!(resp.frame_id, frame_id);
        assert_eq!(resp.intent, "delegate");
    }

    #[test]
    fn fetch_cross_spirit_rejected() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD404));
        seed_frames(&tl, 10, 1);
        let page = LogRecallAdapter::new(Arc::clone(&tl))
            .recall(10, LogRecallFilter::default())
            .unwrap();
        let frame_id = page.entries[0].frame_id;

        let adapter = LogRecallAdapter::new(tl);
        let err = adapter.fetch(20, frame_id).unwrap_err();
        match err {
            LogRecallError::ScopeViolation {
                frame_id: fid,
                requested_pid,
                owner_pid,
            } => {
                assert_eq!(fid, frame_id);
                assert_eq!(requested_pid, 20);
                assert_eq!(owner_pid, 10);
            }
            _ => panic!("expected ScopeViolation"),
        }
    }

    #[test]
    fn recall_emits_capability_invocation() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD405));
        seed_frames(&tl, 10, 3);

        let adapter = LogRecallAdapter::new(Arc::clone(&tl));
        adapter.recall(10, LogRecallFilter::default()).unwrap();

        let audit_filter = FrameFilter {
            kind: Some(FrameKind::CapabilityInvocation),
            ..Default::default()
        };
        let audit_rows = tl.query_frames(audit_filter).unwrap();
        assert!(
            audit_rows.iter().any(|r| r.intent == LOG_RECALL_INTENT),
            "expected CapabilityInvocation row with intent log.recall"
        );
    }

    #[test]
    fn fetch_emits_capability_invocation() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD406));
        seed_frames(&tl, 10, 1);
        let page = LogRecallAdapter::new(Arc::clone(&tl))
            .recall(10, LogRecallFilter::default())
            .unwrap();
        let frame_id = page.entries[0].frame_id;

        let adapter = LogRecallAdapter::new(Arc::clone(&tl));
        adapter.fetch(10, frame_id).unwrap();

        let audit_filter = FrameFilter {
            kind: Some(FrameKind::CapabilityInvocation),
            ..Default::default()
        };
        let audit_rows = tl.query_frames(audit_filter).unwrap();
        assert!(
            audit_rows.iter().any(|r| r.intent == LOG_FETCH_INTENT),
            "expected CapabilityInvocation row with intent log.fetch"
        );
    }

    #[test]
    fn fetch_nonexistent_frame_returns_not_found() {
        let adapter =
            LogRecallAdapter::new(Arc::new(TransparencyLogAdapter::open_in_memory(0xD407)));
        let err = adapter.fetch(10, [0xDE; 16]).unwrap_err();
        assert!(
            matches!(err, LogRecallError::FrameNotFound { frame_id } if frame_id == [0xDE; 16]),
            "expected FrameNotFound, got: {err:?}"
        );
    }

    #[test]
    fn cursor_pagination_no_overlap_and_monotonic_order() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD408));
        seed_frames(&tl, 10, 5);

        let adapter = LogRecallAdapter::new(tl);

        let page1 = adapter
            .recall(10, LogRecallFilter::new(None, None, None, 2, None, None))
            .unwrap();
        assert_eq!(page1.entries.len(), 2);
        let cursor1 = page1.next_cursor.expect("expected next_cursor");

        let page2 = adapter
            .recall(
                10,
                LogRecallFilter::new(None, None, None, 2, Some(cursor1.clone()), None),
            )
            .unwrap();
        assert_eq!(page2.entries.len(), 2);

        // No overlap: every frame_id in page2 must be > every frame_id in page1
        // (since timestamps are monotonically increasing and frame_ids are unique).
        let page1_last_ts = page1.entries.last().unwrap().timestamp_ns;
        let page2_first_ts = page2.entries.first().unwrap().timestamp_ns;
        assert!(
            page2_first_ts >= page1_last_ts,
            "cursor pagination must be monotonic: page2.first_ts ({page2_first_ts}) >= page1.last_ts ({page1_last_ts})"
        );

        // Strict ordering within each page
        for window in page1.entries.windows(2) {
            assert!(
                (window[0].timestamp_ns, window[0].frame_id)
                    < (window[1].timestamp_ns, window[1].frame_id),
                "page1 entries must be strictly ordered"
            );
        }
        for window in page2.entries.windows(2) {
            assert!(
                (window[0].timestamp_ns, window[0].frame_id)
                    < (window[1].timestamp_ns, window[1].frame_id),
                "page2 entries must be strictly ordered"
            );
        }
    }

    #[test]
    fn recall_limit_zero_returns_empty() {
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD409));
        seed_frames(&tl, 10, 3);

        let adapter = LogRecallAdapter::new(tl);
        let page = adapter
            .recall(10, LogRecallFilter::new(None, None, None, 0, None, None))
            .unwrap();
        assert!(page.entries.is_empty());
        assert!(page.next_cursor.is_none());
    }

    struct FixedCrossWallConsent(
        Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError>,
    );

    impl CrossWallRecallConsentPort for FixedCrossWallConsent {
        fn decide(
            &self,
            _team: &TeamId,
            _intent: &str,
        ) -> Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError> {
            self.0.clone()
        }
    }

    struct FixedCrossWallRead;

    impl CrossWallLogReadPort for FixedCrossWallRead {
        fn read_remote(
            &self,
            spirit_pid: u32,
            _remote_team: &TeamId,
            _filter: LogRecallFilter,
        ) -> Result<LogRecallPage, LogRecallError> {
            let entries = if spirit_pid == 20 {
                vec![
                    LogRecallEntry::new(
                        [0xA1; 16],
                        1,
                        DomainFrameKindLabel::TaskAssign,
                        "delegate".into(),
                        spirit_pid,
                        true,
                    ),
                    LogRecallEntry::new(
                        [0xA2; 16],
                        2,
                        DomainFrameKindLabel::TaskAssign,
                        "delegate".into(),
                        spirit_pid,
                        true,
                    ),
                ]
            } else {
                Vec::new()
            };
            Ok(LogRecallPage::new(
                entries,
                None,
                RecallScope::Emitter {
                    reason: EmitterScopeReason::CrossWall,
                },
            ))
        }
    }

    struct AuditObservingRead {
        local: Arc<TransparencyLogAdapter>,
    }

    impl CrossWallLogReadPort for AuditObservingRead {
        fn read_remote(
            &self,
            spirit_pid: u32,
            remote_team: &TeamId,
            filter: LogRecallFilter,
        ) -> Result<LogRecallPage, LogRecallError> {
            let rows = cross_wall_audit_rows(&self.local);
            assert_eq!(
                rows.len(),
                1,
                "intent audit row must exist before remote read"
            );
            let payload: serde_json::Value =
                serde_json::from_slice(&rows[0].payload_redacted).unwrap();
            assert_eq!(payload["remote_team"], remote_team.as_str());
            assert_eq!(payload["outcome"], "disclosing");
            assert_eq!(payload["consent_grant"], CROSS_WALL_RECALL_CONSENT_INTENT);
            assert_eq!(payload["crossed_team_boundary"], false);
            FixedCrossWallRead.read_remote(spirit_pid, remote_team, filter)
        }
    }

    struct RecordingCrossWallConsent {
        result: Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError>,
        calls: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl CrossWallRecallConsentPort for RecordingCrossWallConsent {
        fn decide(
            &self,
            team: &TeamId,
            intent: &str,
        ) -> Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError> {
            self.calls
                .lock()
                .expect("consent call recorder lock should not be poisoned")
                .push((team.to_string(), intent.to_string()));
            self.result.clone()
        }
    }

    #[test]
    fn cross_wall_recall_has_five_distinguishable_outcomes() {
        use maos_domain::log_recall::CrossWallRecallRefusal;

        let remote_team = TeamId::new("team-a").unwrap();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD40A));
        seed_frames(&tl, 10, 1);
        seed_frames(&tl, 20, 2);
        let frame_id = tl
            .query_frames(FrameFilter {
                spirit_pid: Some(10),
                kind: Some(FrameKind::TaskAssign),
                ..Default::default()
            })
            .unwrap()[0]
            .frame_id;
        let local_frame_ids: std::collections::HashSet<_> = tl
            .query_frames(FrameFilter {
                spirit_pid: Some(20),
                kind: Some(FrameKind::TaskAssign),
                ..Default::default()
            })
            .unwrap()
            .into_iter()
            .map(|entry| entry.frame_id)
            .collect();

        let granted_consent = Arc::new(RecordingCrossWallConsent {
            result: Ok(CrossWallRecallConsentDecision::Granted),
            calls: std::sync::Mutex::new(Vec::new()),
        });
        let granted_consent_port: Arc<dyn CrossWallRecallConsentPort> = granted_consent.clone();
        let granted = LogRecallAdapter::new(Arc::clone(&tl))
            .with_cross_wall_consent(granted_consent_port)
            .with_cross_wall_read(Arc::new(FixedCrossWallRead));
        // Granted AND non-empty: the method must route through the remote-read
        // port and return its rows, never delegate to the local `recall`.
        let page = granted
            .recall_cross_wall(20, &remote_team, LogRecallFilter::default())
            .unwrap();
        let returned_frame_ids: std::collections::HashSet<_> =
            page.entries.iter().map(|entry| entry.frame_id).collect();
        assert_eq!(
            returned_frame_ids,
            std::collections::HashSet::from([[0xA1; 16], [0xA2; 16]]),
            "a granted cross-wall recall returns the remote reader's frames"
        );
        assert!(
            returned_frame_ids.is_disjoint(&local_frame_ids),
            "remote disclosure must contain none of the caller-local frames"
        );
        assert!(page.entries.iter().all(|entry| entry.intent == "delegate"));
        // Granted AND legitimately empty: success, not refusal — and never
        // the same observation as a refusal.
        assert!(
            granted
                .recall_cross_wall(30, &remote_team, LogRecallFilter::default())
                .unwrap()
                .entries
                .is_empty(),
            "a legitimately empty page is successful, not refused"
        );
        assert_eq!(
            *granted_consent
                .calls
                .lock()
                .expect("consent call recorder lock should not be poisoned"),
            vec![
                (
                    remote_team.to_string(),
                    CROSS_WALL_RECALL_CONSENT_INTENT.to_owned()
                ),
                (
                    remote_team.to_string(),
                    CROSS_WALL_RECALL_CONSENT_INTENT.to_owned()
                )
            ]
        );

        // ADR-058:52 (`docs/adr/ADR-058-cross-wall-provenance-and-consented-recall.md:52`)
        // — all six typed refusals (missing provider, no grant, reverse-only,
        // stale state, unavailable state, missing read port) must stay
        // distinguishable WITHOUT string matching. Drive all six here and pin
        // their `outcome()` strings distinct in one place. (The test name's
        // "five" predates the read-port refusal; it is kept verbatim because
        // the xtask hermetic legs address this test by exact name.)
        let refusal_reason =
            |result: Result<LogRecallPage, LogRecallError>| -> CrossWallRecallRefusal {
                match result {
                    Err(LogRecallError::ECrossWallRecallDenied { reason, .. }) => reason,
                    Err(_) => panic!("expected a typed cross-wall refusal, got a different error"),
                    Ok(_) => panic!("expected a typed cross-wall refusal, got a page"),
                }
            };

        let no_grant = refusal_reason(
            LogRecallAdapter::new(Arc::clone(&tl))
                .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(Ok(
                    CrossWallRecallConsentDecision::NoGrant,
                ))))
                .recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
        );
        assert!(matches!(no_grant, CrossWallRecallRefusal::NoGrant));

        let wrong_direction = refusal_reason(
            LogRecallAdapter::new(Arc::clone(&tl))
                .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(Ok(
                    CrossWallRecallConsentDecision::WrongDirection,
                ))))
                .recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
        );
        assert!(matches!(
            wrong_direction,
            CrossWallRecallRefusal::WrongDirection
        ));

        let stale = refusal_reason(
            LogRecallAdapter::new(Arc::clone(&tl))
                .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(Err(
                    CrossWallRecallConsentError::Stale {
                        reason: "expired".into(),
                    },
                ))))
                .recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
        );
        assert!(matches!(
            stale,
            CrossWallRecallRefusal::ConsentStateStale(_)
        ));

        let unavailable = refusal_reason(
            LogRecallAdapter::new(Arc::clone(&tl))
                .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(Err(
                    CrossWallRecallConsentError::StateUnavailable {
                        reason: "manifest store offline".into(),
                    },
                ))))
                .recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
        );
        assert!(matches!(
            unavailable,
            CrossWallRecallRefusal::ConsentStateUnavailable(_)
        ));

        // Missing consent provider: no consent port wired at all.
        let no_provider = refusal_reason(LogRecallAdapter::new(Arc::clone(&tl)).recall_cross_wall(
            10,
            &remote_team,
            LogRecallFilter::default(),
        ));
        assert!(matches!(
            no_provider,
            CrossWallRecallRefusal::NoConsentProvider
        ));

        // Missing read port: consent granted but the remote reader was never wired.
        let no_read_port = refusal_reason(
            LogRecallAdapter::new(Arc::clone(&tl))
                .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(Ok(
                    CrossWallRecallConsentDecision::Granted,
                ))))
                .recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
        );
        assert!(matches!(
            no_read_port,
            CrossWallRecallRefusal::ReadPortUnavailable
        ));

        let outcomes = std::collections::HashSet::from([
            no_grant.outcome(),
            wrong_direction.outcome(),
            stale.outcome(),
            unavailable.outcome(),
            no_provider.outcome(),
            no_read_port.outcome(),
        ]);
        assert_eq!(
            outcomes.len(),
            6,
            "ADR-058:52 — all six refusals must be distinguishable without string matching; got {outcomes:?}"
        );

        assert!(matches!(
            granted.fetch(20, frame_id),
            Err(LogRecallError::ScopeViolation { .. })
        ));
    }

    #[test]
    fn cross_wall_recall_journals_refusal() {
        let team = TeamId::new("team-a").unwrap();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD40E));
        let adapter = LogRecallAdapter::new(Arc::clone(&tl)).with_cross_wall_consent(Arc::new(
            FixedCrossWallConsent(Ok(CrossWallRecallConsentDecision::NoGrant)),
        ));
        adapter
            .recall_cross_wall(10, &team, LogRecallFilter::default())
            .unwrap_err();

        let rows = cross_wall_audit_rows(&tl);
        assert_eq!(rows.len(), 1);
        let payload: serde_json::Value = serde_json::from_slice(&rows[0].payload_redacted).unwrap();
        assert_eq!(payload["remote_team"], "team-a");
        assert_eq!(payload["outcome"], "refused_no_grant");
        assert_eq!(payload["consent_grant"], serde_json::Value::Null);
        assert_eq!(payload["crossed_team_boundary"], false);
        assert_eq!(payload["refusal"]["code"], "no_grant");
        assert_eq!(payload["refusal"]["detail"], "no directional grant");
    }

    #[test]
    fn cross_wall_recall_journals_disclosure_before_remote_read() {
        let team = TeamId::new("team-a").unwrap();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD40F));
        let read = Arc::new(AuditObservingRead {
            local: Arc::clone(&tl),
        });
        let page = LogRecallAdapter::new(Arc::clone(&tl))
            .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(Ok(
                CrossWallRecallConsentDecision::Granted,
            ))))
            .with_cross_wall_read(read)
            .recall_cross_wall(20, &team, LogRecallFilter::default())
            .unwrap();
        assert_eq!(page.entries.len(), 2);
        // FR4: a pre-movement "disclosing" intent, then the truthful "disclosed"
        // outcome — never a false "disclosed" before data movement.
        let rows = cross_wall_audit_rows(&tl);
        assert_eq!(rows.len(), 2, "disclosure journals intent then outcome");
        let outcomes: Vec<String> = rows
            .iter()
            .map(|row| {
                serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                    .unwrap()
                    .get("outcome")
                    .and_then(|value| value.as_str())
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(outcomes, vec!["disclosing", "disclosed"]);
    }

    struct FailingCrossWallRead;

    impl CrossWallLogReadPort for FailingCrossWallRead {
        fn read_remote(
            &self,
            _spirit_pid: u32,
            _remote_team: &TeamId,
            _filter: LogRecallFilter,
        ) -> Result<LogRecallPage, LogRecallError> {
            Err(LogRecallError::Storage(
                "simulated remote shard read failure".to_string(),
            ))
        }
    }

    #[test]
    fn cross_wall_recall_journals_failure_when_remote_read_fails() {
        let team = TeamId::new("team-a").unwrap();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD410));
        let adapter = LogRecallAdapter::new(Arc::clone(&tl))
            .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(Ok(
                CrossWallRecallConsentDecision::Granted,
            ))))
            .with_cross_wall_read(Arc::new(FailingCrossWallRead));
        let error = adapter
            .recall_cross_wall(20, &team, LogRecallFilter::default())
            .unwrap_err();
        assert!(
            matches!(error, LogRecallError::Storage(_)),
            "a remote-read failure surfaces as a storage error, not a denial"
        );
        let rows = cross_wall_audit_rows(&tl);
        assert_eq!(rows.len(), 2, "failure journals intent then failure");
        let outcomes: Vec<String> = rows
            .iter()
            .map(|row| {
                serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                    .unwrap()
                    .get("outcome")
                    .and_then(|value| value.as_str())
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(outcomes, vec!["disclosing", "failed"]);
        assert!(
            !outcomes.iter().any(|outcome| *outcome == "disclosed"),
            "a failed read must never leave a false disclosed audit row"
        );
    }

    fn refusal_from(
        consent: Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError>,
    ) -> LogRecallError {
        let team = TeamId::new("team-a").unwrap();
        make_adapter(0xD40D)
            .with_cross_wall_consent(Arc::new(FixedCrossWallConsent(consent)))
            .recall_cross_wall(10, &team, LogRecallFilter::default())
            .unwrap_err()
    }

    #[test]
    fn cross_wall_recall_no_grant_is_observable() {
        let error = refusal_from(Ok(CrossWallRecallConsentDecision::NoGrant));
        assert!(matches!(
            error,
            LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::NoGrant,
                ..
            }
        ));
        assert!(error.to_string().contains("no directional grant"));
    }

    #[test]
    fn cross_wall_recall_wrong_direction_is_observable() {
        let error = refusal_from(Ok(CrossWallRecallConsentDecision::WrongDirection));
        assert!(matches!(
            error,
            LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::WrongDirection,
                ..
            }
        ));
        assert!(error.to_string().contains("reverse directional grant"));
    }

    #[test]
    fn cross_wall_recall_stale_state_is_observable() {
        let error = refusal_from(Err(CrossWallRecallConsentError::Stale {
            reason: "expired".into(),
        }));
        assert!(matches!(
            error,
            LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::ConsentStateStale(_),
                ..
            }
        ));
        assert!(error
            .to_string()
            .contains("consent state is stale: expired"));
    }

    #[test]
    fn cross_wall_recall_unavailable_state_is_observable() {
        let error = refusal_from(Err(CrossWallRecallConsentError::StateUnavailable {
            reason: "offline".into(),
        }));
        assert!(matches!(
            error,
            LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::ConsentStateUnavailable(_),
                ..
            }
        ));
        assert!(error
            .to_string()
            .contains("consent state is unavailable: offline"));
    }

    #[test]
    fn cross_wall_recall_granted_without_read_port_fails_closed() {
        let team = TeamId::new("team-a").unwrap();
        let adapter = make_adapter(0xD40C).with_cross_wall_consent(Arc::new(
            FixedCrossWallConsent(Ok(CrossWallRecallConsentDecision::Granted)),
        ));
        let error = adapter
            .recall_cross_wall(10, &team, LogRecallFilter::default())
            .unwrap_err();
        assert!(matches!(
            error,
            LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::ReadPortUnavailable,
                ..
            }
        ));
        assert!(error
            .to_string()
            .contains("cross-wall read port is not wired"));
    }

    #[test]
    fn cross_wall_recall_without_injected_consent_fails_closed() {
        let team = TeamId::new("team-a").unwrap();
        let adapter = make_adapter(0xD40B);
        let error = adapter
            .recall_cross_wall(10, &team, LogRecallFilter::default())
            .unwrap_err();
        assert!(matches!(
            error,
            LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::NoConsentProvider,
                ..
            }
        ));
        assert!(error.to_string().contains("consent provider is not wired"));
    }
}
