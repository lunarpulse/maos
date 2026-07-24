#![forbid(unsafe_code)]

//! Log-recall adapter — implements `LogRecallPort` over the Transparency Log.
//!
//! Provides participant-scoped (emitter-side at v0.3-β), cursor-paginated
//! read-side access with on-demand payload fetch and A2A consent honoring.
//!
//! # v0.3-β scope
//!
//! Only the **emitter** Spirit can recall/fetch their own frames. Recipient-side
//! participation requires a `transparency_log_recipients` companion table
//! (deferred to v0.5+, Story 8.2 or 9.1).
//!
//! # A2A consent envelope
//!
//! When an entry's payload would carry a `ConsentEnvelope`, the adapter checks
//! `consent_envelope.valid_until > now_ns()`. At v0.3-β with
//! `consent_envelope == None` on every TL row, the check is structurally a
//! no-op AND a comment block documents the v0.5 binding contract from §7.1 +
//! Story 6.3. When Story 6.3 wires the envelope, this method gains the
//! `consent_envelope.valid_until > now_ns()` runtime check and the v0.3
//! scaffold-comment converts to runtime enforcement without API change.

use std::collections::HashSet;
use std::sync::Arc;

#[cfg(feature = "spirit_test")]
use maos_spirit_sdk::spirit_test::{AttemptResult, IsolationHookPoint, ObservationResult};
#[cfg(feature = "spirit_test")]
use parking_lot::Mutex;

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{
    CrossWallRecallRefusal, FrameKindLabel as DomainFrameKindLabel, LogFetchResponse,
    LogRecallCursor, LogRecallEntry, LogRecallError, LogRecallFilter, LogRecallPage,
};
use maos_domain::ports::{
    CrossWallRecallConsentDecision, CrossWallRecallConsentError, CrossWallRecallConsentPort,
    LogRecallPort,
};
use maos_domain::team::TeamId;

use super::transparency_log::{
    FrameFilter, FrameKind, TransparencyLogAdapter, TransparencyLogEntry,
};

/// Stable intent string constant for `CapabilityInvocation` audit row.
pub const LOG_RECALL_INTENT: &str = "log.recall";
/// Canonical A2A intent used for signed directional cross-team grants.
pub const CROSS_WALL_RECALL_CONSENT_INTENT: &str = "log:recall";

/// Stable intent string constant for `CapabilityInvocation` audit row.
pub const LOG_FETCH_INTENT: &str = "log.fetch";

/// Log-recall adapter — stateless composer over `Arc<TransparencyLogAdapter>`.
///
/// Does NOT require `#[i9_exempt]` — holds only an `Arc` reference to
/// an existing I9-sanctioned holder (same shape as `SelfTelemetryAggregator`).
pub struct LogRecallAdapter {
    transparency_log: Arc<TransparencyLogAdapter>,
    cross_wall_consent: Option<Arc<dyn CrossWallRecallConsentPort>>,
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
            cross_wall_consent: None,
            #[cfg(feature = "spirit_test")]
            isolation_hook: None,
        }
    }

    /// Attach the directional cross-wall consent decision seam. The builder is
    /// unconditional; `new` remains one-argument for all existing consumers.
    pub fn with_cross_wall_consent(mut self, consent: Arc<dyn CrossWallRecallConsentPort>) -> Self {
        self.cross_wall_consent = Some(consent);
        self
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
    /// Returns `None` if the domain label has no kernel-side equivalent — the
    /// caller must treat this as "no kind filter" rather than silently routing
    /// to a default kind (Story 5.5d Finding #11 — the previous `_ =>
    /// FrameKind::McpInvocation` catch-all silently misclassified future
    /// `FrameKindLabel` variants in audit recall).
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
            other => {
                eprintln!(
                    "maos: warning: unmapped FrameKindLabel {:?} in to_kernel_kind \
                     — treating as no-kind-filter (returning None) to avoid \
                     silent misclassification",
                    other
                );
                None
            }
        }
    }

    /// Build a domain `LogRecallEntry` from a kernel-side `TransparencyLogEntry`.
    fn build_entry(tl_entry: &TransparencyLogEntry) -> LogRecallEntry {
        LogRecallEntry::new(
            tl_entry.frame_id,
            tl_entry.timestamp_ns,
            Self::to_domain_kind(tl_entry.kind),
            tl_entry.intent.clone(),
            tl_entry.spirit_pid,
            !tl_entry.payload_redacted.is_empty(),
        )
    }

    /// Apply cursor pagination to a sorted result set.
    /// `entries` must be ordered by `(timestamp_ns ASC, frame_id ASC)`.
    fn apply_cursor(
        entries: &[TransparencyLogEntry],
        cursor: Option<&LogRecallCursor>,
        limit: usize,
    ) -> (Vec<TransparencyLogEntry>, Option<LogRecallCursor>) {
        if limit == 0 {
            return (vec![], None);
        }

        let start_idx = match cursor {
            Some(c) => {
                // Skip entries until we pass the cursor position
                entries
                    .iter()
                    .position(|e| {
                        (e.timestamp_ns, &e.frame_id) > (c.last_timestamp_ns, &c.last_frame_id)
                    })
                    .unwrap_or(entries.len())
            }
            None => 0,
        };

        // Request limit+1 to detect whether a next_cursor is needed
        let fetch_limit = limit.saturating_add(1);
        let end_idx = (start_idx + fetch_limit).min(entries.len());

        let selected: Vec<_> = entries[start_idx..end_idx].iter().cloned().collect();

        let next_cursor = if selected.len() > limit {
            // We fetched one extra — there IS a next page
            let last = &selected[limit - 1];
            Some(LogRecallCursor::new(last.timestamp_ns, last.frame_id))
        } else {
            None
        };

        // Return at most `limit` entries
        let result = if selected.len() > limit {
            selected[..limit].to_vec()
        } else {
            selected
        };

        (result, next_cursor)
    }

    /// Convert kernel origin → domain origin.
    fn to_domain_origin(origin: FrameOrigin) -> maos_domain::invariants::i3::FrameOrigin {
        origin
    }
}

impl LogRecallPort for LogRecallAdapter {
    fn recall(
        &self,
        spirit_pid: u32,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
        // 1. Emit CapabilityInvocation audit row BEFORE data-movement (FR4).
        let audit_payload = serde_json::json!({
            "limit": filter.limit,
            "cursor_present": filter.cursor.is_some(),
        });
        let audit_payload_str = audit_payload.to_string();
        let _token = self.transparency_log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            LOG_RECALL_INTENT,
            audit_payload_str.as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        // 1a. Spirit_test isolation hook (Story 4.4 → Story 4.5 corpus plug).
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&format!("log.recall:{spirit_pid}"), true);

        // 2. Build kernel-side filter.
        let kind_filter = filter.kind.as_ref().and_then(Self::to_kernel_kind);
        let kernel_filter = FrameFilter {
            spirit_pid: Some(spirit_pid), // emitter-scope at v0.3-β
            frame_id: None,
            kind: kind_filter,
            correlation_id: None,
            since_ns: filter.since_ns,
            until_ns: filter.until_ns,
            limit: Some(
                filter
                    .limit
                    .min(LogRecallFilter::MAX_LIMIT)
                    .saturating_add(1),
            ),
            cursor_timestamp_ns: filter.cursor.as_ref().map(|c| c.last_timestamp_ns),
            cursor_frame_id: filter.cursor.as_ref().map(|c| c.last_frame_id),
        };

        // 3. Query transparency log with SQL-level cursor pagination.
        let all_entries = self
            .transparency_log
            .query_frames(kernel_filter)
            .map_err(|e| LogRecallError::Storage(e.to_string()))?;

        // 3a. Exclude CapabilityInvocation audit rows from recall results —
        // they are audit metadata, not visible business frames.
        let audit_filtered: Vec<_> = all_entries
            .into_iter()
            .filter(|e| e.kind != FrameKind::CapabilityInvocation)
            .collect();

        // 4. Apply intent filter (post-query since TL query_frames does not support it).
        let intent_filtered: Vec<_> = if let Some(ref intent_f) = filter.intent_filter {
            audit_filtered
                .into_iter()
                .filter(|e| e.intent == *intent_f)
                .collect()
        } else {
            audit_filtered
        };

        // 5. Apply cursor pagination (limit-clamp already pushed to SQL as limit+1).
        let effective_limit = filter.limit.min(LogRecallFilter::MAX_LIMIT);
        let (selected, next_cursor) =
            Self::apply_cursor(&intent_filtered, filter.cursor.as_ref(), effective_limit);

        // 6. Build domain entries.
        let entries: Vec<LogRecallEntry> = selected.iter().map(Self::build_entry).collect();

        Ok(LogRecallPage::new(entries, next_cursor))
    }

    fn recall_cross_wall(
        &self,
        spirit_pid: u32,
        team: &TeamId,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
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
            return Err(LogRecallError::ECrossWallRecallDenied {
                team: team.clone(),
                reason,
            });
        }
        self.recall(spirit_pid, filter)
    }

    fn fetch(
        &self,
        spirit_pid: u32,
        frame_id: [u8; 16],
    ) -> Result<LogFetchResponse, LogRecallError> {
        // 1. Emit CapabilityInvocation audit row BEFORE data-movement (FR4).
        let frame_id_hex: String = frame_id.iter().map(|b| format!("{b:02x}")).collect();
        let audit_payload = serde_json::json!({
            "frame_id_hex": frame_id_hex,
        });
        let audit_payload_str = audit_payload.to_string();
        let _token = self.transparency_log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            LOG_FETCH_INTENT,
            audit_payload_str.as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        // 1a. Spirit_test isolation hook (Story 4.4 → Story 4.5 corpus plug).
        #[cfg(feature = "spirit_test")]
        {
            let hex_prefix: String = frame_id
                .iter()
                .take(4)
                .map(|b| format!("{b:02x}"))
                .collect();
            self.fire_isolation_hooks(&format!("log.fetch:{spirit_pid}:{hex_prefix}"), true);
        }

        // 2. Query transparency log for the specific frame by primary key.
        let entry = self
            .transparency_log
            .query_frame_by_id(frame_id)
            .map_err(|e| LogRecallError::Storage(e.to_string()))?
            .ok_or(LogRecallError::FrameNotFound { frame_id })?;

        // 3. Emitter-scope check (v0.3-β).
        if entry.spirit_pid != spirit_pid {
            return Err(LogRecallError::ScopeViolation {
                frame_id,
                requested_pid: spirit_pid,
                owner_pid: entry.spirit_pid,
            });
        }

        // 4. A2A consent envelope honoring.
        // v0.3-β: ConsentEnvelope is None on every TL row today.
        // When Story 6.3 wires the envelope, this block gains:
        //   if let Some(envelope) = &frame.consent_envelope {
        //       if envelope.valid_until <= wall_clock_now_ns() {
        //           return Err(LogRecallError::InvalidCursor("consent envelope expired".into()));
        //       }
        //   }
        // The I8/§7.1 forward-looking shape is a no-op pass-through at v0.3-β.

        Ok(LogFetchResponse::new(
            entry.frame_id,
            entry.timestamp_ns,
            Self::to_domain_kind(entry.kind),
            entry.intent.clone(),
            entry.payload_redacted.clone(),
            entry.capability_token,
            Self::to_domain_origin(entry.origin),
        ))
    }
}

#[cfg(test)]
mod tests {
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

        let granted_consent = Arc::new(RecordingCrossWallConsent {
            result: Ok(CrossWallRecallConsentDecision::Granted),
            calls: std::sync::Mutex::new(Vec::new()),
        });
        let granted_consent_port: Arc<dyn CrossWallRecallConsentPort> = granted_consent.clone();
        let granted =
            LogRecallAdapter::new(Arc::clone(&tl)).with_cross_wall_consent(granted_consent_port);
        // Granted AND non-empty: the method must forward spirit_pid/filter to
        // `recall` and return the frames — an implementation that consents
        // correctly but always returns an empty page fails here.
        let page = granted
            .recall_cross_wall(20, &remote_team, LogRecallFilter::default())
            .unwrap();
        assert_eq!(
            page.entries.len(),
            2,
            "a granted cross-wall recall returns the target pid's frames"
        );
        assert!(page.entries.iter().all(|e| e.intent == "delegate"));
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

        let denied = LogRecallAdapter::new(Arc::clone(&tl)).with_cross_wall_consent(Arc::new(
            FixedCrossWallConsent(Ok(CrossWallRecallConsentDecision::NoGrant)),
        ));
        assert!(matches!(
            denied.recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
            Err(LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::NoGrant,
                ..
            })
        ));

        let wrong_direction =
            LogRecallAdapter::new(Arc::clone(&tl)).with_cross_wall_consent(Arc::new(
                FixedCrossWallConsent(Ok(CrossWallRecallConsentDecision::WrongDirection)),
            ));
        assert!(matches!(
            wrong_direction.recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
            Err(LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::WrongDirection,
                ..
            })
        ));

        let stale = LogRecallAdapter::new(Arc::clone(&tl)).with_cross_wall_consent(Arc::new(
            FixedCrossWallConsent(Err(CrossWallRecallConsentError::Stale {
                reason: "expired".into(),
            })),
        ));
        assert!(matches!(
            stale.recall_cross_wall(10, &remote_team, LogRecallFilter::default()),
            Err(LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::ConsentStateStale(_),
                ..
            })
        ));

        assert!(matches!(
            granted.fetch(20, frame_id),
            Err(LogRecallError::ScopeViolation { .. })
        ));
    }

    #[test]
    fn cross_wall_recall_without_injected_consent_fails_closed() {
        let team = TeamId::new("team-a").unwrap();
        let adapter = make_adapter(0xD40B);
        assert!(matches!(
            adapter.recall_cross_wall(10, &team, LogRecallFilter::default()),
            Err(LogRecallError::ECrossWallRecallDenied {
                reason: CrossWallRecallRefusal::NoConsentProvider,
                ..
            })
        ));
    }
}
