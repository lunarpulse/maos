#![cfg(feature = "network")]
#![forbid(unsafe_code)]

use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use maos_bin::recall_mediation::MediatedLogRecall;
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::log_recall::{FrameKindLabel, LogRecallError, LogRecallFilter};
use maos_domain::ports::LogRecallPort;
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::transparency_log::{
    capability_token_column, FrameFilter, FrameKind, TransparencyLogAdapter, TransparencyLogEntry,
};
use maos_kernel_core::capability::{
    cap_audit::{self, CapAuditEvent},
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner},
    cap_quota::{CapQuotaTracker, IssuanceQuota, QuotaClock},
    cap_tokens::{self, Ed25519SigningKey},
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::security::SecurityManagerAdapter;

const NONCE: u64 = 0x17_03_d;
const PID: u32 = 42;
const OWN: [u8; 16] = [0x31; 16];
const FOREIGN: [u8; 16] = [0x32; 16];
const PAYLOAD: &[u8] = b"recall mediation payload";

struct World {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    recall: MediatedLogRecall,
    audit: tokio::sync::mpsc::Receiver<CapAuditEvent>,
    adapter_calls: Arc<AtomicUsize>,
}

impl World {
    fn new(scopes: Vec<Scope>) -> Self {
        Self::with_tracker(
            scopes,
            CapQuotaTracker::new(),
            Arc::new(TransparencyLogAdapter::open_in_memory(NONCE)),
        )
    }

    fn with_tracker(
        scopes: Vec<Scope>,
        quota: CapQuotaTracker,
        tl: Arc<TransparencyLogAdapter>,
    ) -> Self {
        cap_tokens::init_monotonic_base();
        let adapter_calls = Arc::new(AtomicUsize::new(0));
        let policy = Arc::new(PolicyTable::new());
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            PID,
            ManifestCapabilityScope {
                scopes,
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        inner.spirit_postures.insert(
            PID,
            maos_kernel_core::security::posture::PostureState {
                current: maos_kernel_core::security::manifest::Posture::AutonomousWithHalt,
                allowed_max: maos_kernel_core::security::manifest::Posture::AutonomousWithHalt,
                epistemic_policy: maos_kernel_core::security::manifest::EpistemicPolicySection {
                    rules: vec![],
                    default_action:
                        maos_kernel_core::security::manifest::EpistemicAction::VerbalizeOnly,
                },
            },
        );
        policy.update(inner);
        let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
        let (audit_tx, audit) = cap_audit::channel();
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            Ed25519SigningKey::new([0x17; 32]),
            NONCE,
            policy,
            audit_tx,
            quota,
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ));
        let recall = MediatedLogRecall::new(
            Arc::new(LogRecallAdapter::new(Arc::clone(&tl)).with_consent_clock({
                let calls = Arc::clone(&adapter_calls);
                Arc::new(move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    maos_domain::frame::consent_wall_now_ns()
                })
            })),
            Arc::clone(&capability),
            security,
            Arc::clone(&tl),
            None,
            None,
            "recall-test".into(),
        );
        Self {
            tl,
            capability,
            recall,
            audit,
            adapter_calls,
        }
    }

    fn granted() -> Self {
        Self::new(vec![Scope::LogRecall, Scope::LogFetch])
    }

    fn seed(&self, id: [u8; 16], pid: u32, payload: &[u8]) {
        let _logged = self.tl.insert_frame_event_with_id(
            Some(id),
            FrameKind::TaskAssign,
            pid,
            "recall-test",
            "peer",
            None,
            "Standard",
            payload,
            FrameOrigin::SpiritAuto,
        );
    }

    fn operation_rows(&self) -> Vec<TransparencyLogEntry> {
        self.tl
            .query_frames(FrameFilter {
                order_by_insertion: true,
                ..Default::default()
            })
            .unwrap()
            .into_iter()
            .filter(|row| row.intent.starts_with("log."))
            .collect()
    }

    fn drain_audit(&mut self) -> Vec<CapAuditEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.audit.try_recv() {
            events.push(event);
        }
        events
    }

    fn assert_no_mint(&mut self) {
        assert!(!self.capability.has_active_tokens_for_pid(PID));
        assert!(
            self.drain_audit()
                .iter()
                .all(|event| !matches!(event, CapAuditEvent::Issue { .. })),
            "validation/denial must not issue even an immediately revoked token"
        );
    }

    fn assert_issued_and_revoked(&mut self, row: &TransparencyLogEntry, scope: Scope) {
        let events = self.drain_audit();
        let issued: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                CapAuditEvent::Issue {
                    token_id,
                    spirit_pid,
                    scope: issued_scope,
                    ..
                } => {
                    assert_eq!(*spirit_pid, PID);
                    assert_eq!(*issued_scope, scope);
                    Some(token_id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(issued.len(), 1);
        assert_eq!(
            row.capability_token,
            Some(capability_token_column(issued[0]))
        );
        assert!(events.iter().any(|event| matches!(event,
            CapAuditEvent::Revoke { token_id, .. } if token_id == issued[0])));
        assert!(!self.capability.has_active_tokens_for_pid(PID));
    }
}

#[derive(Debug, Default)]
struct ManualClock(AtomicU64);

impl QuotaClock for ManualClock {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

impl ManualClock {
    fn advance(&self) {
        self.0.fetch_add(1_000_000_000, Ordering::SeqCst);
    }
}

fn small_quota(budget: u64, clock: &Arc<ManualClock>) -> CapQuotaTracker {
    CapQuotaTracker::with_clock(
        IssuanceQuota {
            budget_per_window: budget,
            window: Duration::from_secs(1),
        },
        Arc::clone(clock) as _,
    )
}

fn quota_world(budget: u64, clock: &Arc<ManualClock>) -> World {
    World::with_tracker(
        vec![Scope::LogRecall, Scope::LogFetch],
        small_quota(budget, clock),
        Arc::new(TransparencyLogAdapter::open_in_memory(NONCE)),
    )
}

fn assert_call(row: &TransparencyLogEntry, op: &str) {
    assert_eq!(row.kind as i64, 7);
    assert_eq!(row.intent, format!("log.{op}"));
    assert_eq!(row.spirit_pid, PID);
    assert_eq!(row.boot_nonce, NONCE);
    let token = row
        .capability_token
        .expect("call row carries the actual token column");
    assert_ne!(&token[..16], &[0; 16]);
    assert_eq!(&token[16..], &[0; 16]);
}

fn assert_receipt(
    row: &TransparencyLogEntry,
    op: &str,
    code: &str,
    frame: Option<[u8; 16]>,
    owner: Option<u32>,
) {
    assert_eq!(row.kind as i64, 28);
    assert_eq!(row.intent, format!("log.{op}.refused:{code}"));
    assert_eq!(row.spirit_pid, PID);
    assert_eq!(row.boot_nonce, NONCE);
    assert_eq!(row.capability_token, None);
    let mut expected = format!("{{\"op\":\"{op}\",\"code\":\"{code}\"");
    if let Some(id) = frame {
        let hex = id
            .chunks_exact(2)
            .map(hex::encode)
            .collect::<Vec<_>>()
            .join(":");
        expected.push_str(&format!(",\"frame_id_hex\":\"{hex}\""));
    }
    if let Some(pid) = owner {
        expected.push_str(&format!(",\"owner_pid\":{pid}"));
    }
    expected.push_str(&format!(",\"boot_nonce\":{NONCE}}}"));
    assert_eq!(
        std::str::from_utf8(&row.payload_redacted).unwrap(),
        expected,
        "receipt key order is a byte-level contract, not JSON map equivalence"
    );
}

#[test]
fn granted_recall_has_token_column_without_refusal_receipt() {
    let mut world = World::granted();
    world.seed(OWN, PID, PAYLOAD);
    world.seed(FOREIGN, 0, b"kernel private");
    let page = world
        .recall
        .recall(PID, LogRecallFilter::default())
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].frame_id, OWN);
    assert_eq!(page.entries[0].kind, FrameKindLabel::TaskAssign);
    assert!(page.entries[0].payload_available);
    assert!(page.next_cursor.is_none());
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    assert_call(&rows[0], "recall");
    world.assert_issued_and_revoked(&rows[0], Scope::LogRecall);
}

#[test]
fn granted_own_fetch_returns_payload_and_token_bearing_call() {
    let mut world = World::granted();
    world.seed(OWN, PID, PAYLOAD);
    let fetched = world.recall.fetch(PID, OWN).unwrap();
    assert_eq!(fetched.frame_id, OWN);
    assert_eq!(fetched.kind, FrameKindLabel::TaskAssign);
    assert_eq!(fetched.payload_redacted, PAYLOAD);
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    assert_call(&rows[0], "fetch");
    world.assert_issued_and_revoked(&rows[0], Scope::LogFetch);
}

#[test]
fn foreign_pid_zero_fetch_logs_one_call_before_exact_scope_receipt() {
    let mut world = World::granted();
    world.seed(FOREIGN, 0, b"kernel private");
    assert_eq!(
        world.recall.fetch(PID, FOREIGN),
        Err(LogRecallError::ScopeViolation {
            frame_id: FOREIGN,
            requested_pid: PID,
            owner_pid: 0,
        })
    );
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 2);
    assert_call(&rows[0], "fetch");
    assert_receipt(&rows[1], "fetch", "scope-violation", Some(FOREIGN), Some(0));
    assert!(rows[0].insertion_id < rows[1].insertion_id);
    world.assert_issued_and_revoked(&rows[0], Scope::LogFetch);
}

#[test]
fn ungranted_operations_emit_only_capability_denied_receipts() {
    for fetch in [false, true] {
        let mut world = World::new(vec![]);
        world.seed(OWN, PID, PAYLOAD);
        if fetch {
            assert_eq!(
                world.recall.fetch(PID, OWN),
                Err(LogRecallError::CapabilityDenied)
            );
        } else {
            assert_eq!(
                world.recall.recall(PID, LogRecallFilter::default()),
                Err(LogRecallError::CapabilityDenied)
            );
        }
        let rows = world.operation_rows();
        assert_eq!(rows.len(), 1);
        assert_receipt(
            &rows[0],
            if fetch { "fetch" } else { "recall" },
            "capability-denied",
            fetch.then_some(OWN),
            None,
        );
        world.assert_no_mint();
    }
}

#[test]
fn limit_1025_is_refused_before_mint_or_adapter_call() {
    let mut world = World::granted();
    // The constructor clamps; use the public shape to exercise the hostile boundary.
    let filter = LogRecallFilter {
        limit: 1025,
        ..Default::default()
    };
    assert_eq!(
        world.recall.recall(PID, filter),
        Err(LogRecallError::LimitExceeded {
            requested: 1025,
            max: 1024,
        })
    );
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    assert_receipt(&rows[0], "recall", "limit-exceeded", None, None);
    world.assert_no_mint();
}

#[test]
fn empty_page_is_success_without_refusal_receipt() {
    let mut world = World::granted();
    let page = world
        .recall
        .recall(PID, LogRecallFilter::default())
        .unwrap();
    assert!(page.entries.is_empty());
    assert!(page.next_cursor.is_none());
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    assert_call(&rows[0], "recall");
    world.assert_issued_and_revoked(&rows[0], Scope::LogRecall);
}

#[test]
fn empty_payload_fetch_is_success_without_refusal_receipt() {
    let mut world = World::granted();
    world.seed(OWN, PID, b"");
    assert!(world
        .recall
        .fetch(PID, OWN)
        .unwrap()
        .payload_redacted
        .is_empty());
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    assert_call(&rows[0], "fetch");
    world.assert_issued_and_revoked(&rows[0], Scope::LogFetch);
}

#[test]
fn absent_posture_is_capability_denied_without_mint() {
    let mut world = World::granted();
    assert_eq!(
        world.recall.recall(PID + 1, LogRecallFilter::default()),
        Err(LogRecallError::CapabilityDenied)
    );
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spirit_pid, PID + 1);
    assert_eq!(rows[0].intent, "log.recall.refused:capability-denied");
    assert_eq!(rows[0].kind as i64, 28);
    world.assert_no_mint();
}

#[test]
fn native_timestamp_validation_receipt_keeps_detail_before_boot_nonce() {
    for filter in [
        LogRecallFilter {
            since_ns: Some(u64::MAX),
            ..Default::default()
        },
        LogRecallFilter {
            until_ns: Some(u64::MAX),
            ..Default::default()
        },
        LogRecallFilter {
            cursor: Some(maos_domain::log_recall::LogRecallCursor::new(u64::MAX, OWN)),
            ..Default::default()
        },
    ] {
        let mut world = World::granted();
        assert!(matches!(
            world.recall.recall(PID, filter),
            Err(LogRecallError::InvalidCursor(_))
        ));
        let rows = world.operation_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(std::str::from_utf8(&rows[0].payload_redacted).unwrap(),
            format!("{{\"op\":\"recall\",\"code\":\"invalid-request\",\"detail\":\"timestamp exceeds i64::MAX\",\"boot_nonce\":{NONCE}}}"));
        world.assert_no_mint();
    }
}

#[cfg(feature = "wasm-host")]
mod wire {
    use super::*;
    use maos_wasm_host::recall_wire::{
        self, RecallReply, RecallRequest, WireBytes, WireCursor, WireError, WireFilter,
    };

    fn filter() -> WireFilter {
        WireFilter {
            kind: None,
            since_ns: None,
            until_ns: None,
            limit: 10,
            cursor: None,
            intent_filter: None,
        }
    }

    fn serve(world: &World, request: &RecallRequest) -> RecallReply {
        let body = recall_wire::encode_request(request).unwrap();
        recall_wire::decode_reply(&world.recall.serve_wire(PID, &body).unwrap()).unwrap()
    }

    #[test]
    fn hostile_unknown_tag_and_junk_return_protocol_error_without_mint() {
        for body in [vec![0xd9, 0xea, 0x62, 0xf6], vec![0xff], vec![]] {
            let mut world = World::granted();
            assert!(world.recall.serve_wire(PID, &body).is_err());
            // Session, not serve_wire, journals the channel-closure receipt.
            assert!(world.operation_rows().is_empty());
            world.assert_no_mint();
        }
        let mut world = World::granted();
        let mut trailing = recall_wire::encode_request(&RecallRequest::Fetch {
            frame_id: WireBytes(OWN.to_vec()),
        })
        .unwrap();
        trailing.push(0xf6);
        assert!(world.recall.serve_wire(PID, &trailing).is_err());
        assert!(world.operation_rows().is_empty());
        world.assert_no_mint();
    }

    #[test]
    fn malformed_wire_id_kind_and_timestamps_refuse_without_mint() {
        let mut requests = vec![RecallRequest::Fetch {
            frame_id: WireBytes(vec![1; 15]),
        }];
        let mut invalid_kind = filter();
        invalid_kind.kind = Some("not-a-frame-kind".into());
        requests.push(RecallRequest::Recall {
            filter: invalid_kind,
        });
        let mut overflow = filter();
        overflow.since_ns = Some(u64::MAX);
        requests.push(RecallRequest::Recall { filter: overflow });
        let mut cursor = filter();
        cursor.cursor = Some(WireCursor {
            last_timestamp_ns: 1,
            last_frame_id: WireBytes(vec![1; 17]),
        });
        requests.push(RecallRequest::Recall { filter: cursor });
        for request in requests {
            let mut world = World::granted();
            assert!(matches!(
                serve(&world, &request),
                RecallReply::Error(WireError::InvalidRequest(_))
            ));
            let rows = world.operation_rows();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].kind as i64, 28);
            assert!(rows[0].intent.ends_with(".refused:invalid-request"));
            world.assert_no_mint();
        }
    }
    #[test]
    fn reversed_representable_interval_returns_empty_page_without_receipt() {
        let mut world = World::granted();
        world.seed(OWN, PID, PAYLOAD);
        let mut reversed = filter();
        reversed.since_ns = Some(2);
        reversed.until_ns = Some(1);
        assert_eq!(
            serve(&world, &RecallRequest::Recall { filter: reversed }),
            RecallReply::Page {
                entries: vec![],
                next_cursor: None
            }
        );
        let rows = world.operation_rows();
        assert_eq!(rows.len(), 1);
        assert_call(&rows[0], "recall");
        world.assert_issued_and_revoked(&rows[0], Scope::LogRecall);
    }

    #[test]
    fn oversized_stored_payload_returns_reply_too_large_and_next_fetch_succeeds() {
        let mut world = World::granted();
        let oversized = vec![b'x'; recall_wire::REPLY_BYTE_BOUND as usize + 1];
        world.seed(OWN, PID, &oversized);
        world.seed(FOREIGN, PID, PAYLOAD);
        assert_eq!(
            serve(
                &world,
                &RecallRequest::Fetch {
                    frame_id: WireBytes(OWN.to_vec())
                }
            ),
            RecallReply::Error(WireError::ReplyTooLarge(recall_wire::REPLY_BYTE_BOUND))
        );
        let rows = world.operation_rows();
        assert_eq!(rows.len(), 2);
        assert_call(&rows[0], "fetch");
        assert_receipt(&rows[1], "fetch", "reply-too-large", Some(OWN), None);
        assert!(rows[0].insertion_id < rows[1].insertion_id);
        world.assert_issued_and_revoked(&rows[0], Scope::LogFetch);
        match serve(
            &world,
            &RecallRequest::Fetch {
                frame_id: WireBytes(FOREIGN.to_vec()),
            },
        ) {
            RecallReply::Fetched {
                frame_id,
                payload,
                kind,
                origin,
                ..
            } => {
                assert_eq!(frame_id.0, FOREIGN);
                assert_eq!(payload.0, PAYLOAD);
                assert_eq!(kind, "TaskAssign");
                assert_eq!(origin, "spirit-auto");
            }
            other => panic!("next request must remain usable: {other:?}"),
        }
        let rows = world.operation_rows();
        assert_eq!(rows.len(), 3);
        assert_call(&rows[2], "fetch");
        world.assert_issued_and_revoked(&rows[2], Scope::LogFetch);
    }

    #[test]
    fn fetched_envelope_over_budget_and_unencodable_first_entry_are_refused_once() {
        let mut world = World::granted();
        world.seed(
            OWN,
            PID,
            &vec![b'x'; recall_wire::REPLY_BYTE_BOUND as usize],
        );
        assert_eq!(
            serve(
                &world,
                &RecallRequest::Fetch {
                    frame_id: WireBytes(OWN.to_vec())
                }
            ),
            RecallReply::Error(WireError::ReplyTooLarge(recall_wire::REPLY_BYTE_BOUND))
        );
        let rows = world.operation_rows();
        assert_eq!(rows.len(), 2);
        assert_receipt(&rows[1], "fetch", "reply-too-large", Some(OWN), None);
        world.assert_issued_and_revoked(&rows[0], Scope::LogFetch);

        let mut world = World::granted();
        world.tl.insert_frame_event(
            FrameKind::TaskAssign,
            PID,
            None,
            &"x".repeat(recall_wire::REPLY_BYTE_BOUND as usize),
            PAYLOAD,
            FrameOrigin::SpiritAuto,
        );
        assert_eq!(
            serve(&world, &RecallRequest::Recall { filter: filter() }),
            RecallReply::Error(WireError::ReplyTooLarge(recall_wire::REPLY_BYTE_BOUND))
        );
        let rows = world.operation_rows();
        assert_eq!(rows.len(), 2);
        assert_receipt(&rows[1], "recall", "reply-too-large", None, None);
        world.assert_issued_and_revoked(&rows[0], Scope::LogRecall);
    }
}

fn assert_exhausted_operation(fetch: bool) {
    let clock = Arc::new(ManualClock::default());
    let quota = small_quota(1, &clock);
    let _reservation = quota.charge(PID).unwrap();
    let mut world = World::with_tracker(
        vec![Scope::LogRecall, Scope::LogFetch],
        quota,
        Arc::new(TransparencyLogAdapter::open_in_memory(NONCE)),
    );
    world.seed(OWN, PID, PAYLOAD);
    if fetch {
        assert_eq!(
            world.recall.fetch(PID, OWN),
            Err(LogRecallError::CapabilityDenied)
        );
    } else {
        assert_eq!(
            world.recall.recall(PID, LogRecallFilter::default()),
            Err(LogRecallError::CapabilityDenied)
        );
    }
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    assert_receipt(
        &rows[0],
        if fetch { "fetch" } else { "recall" },
        "capability-denied",
        fetch.then_some(OWN),
        None,
    );
    assert_eq!(world.adapter_calls.load(Ordering::SeqCst), 0);
    world.assert_no_mint();
}

#[test]
fn exhausted_recall_refuses_before_adapter_with_one_receipt() {
    assert_exhausted_operation(false);
}

#[test]
fn exhausted_fetch_refuses_before_adapter_with_one_receipt() {
    assert_exhausted_operation(true);
}

#[test]
fn budget_one_call_consumes_one_issuance_and_revoke_does_not_refund() {
    let clock = Arc::new(ManualClock::default());
    let mut world = quota_world(1, &clock);
    world.seed(OWN, PID, PAYLOAD);
    assert_eq!(
        world.recall.fetch(PID, OWN).unwrap().payload_redacted,
        PAYLOAD
    );
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 1);
    world.assert_issued_and_revoked(&rows[0], Scope::LogFetch);
    assert_eq!(
        world.recall.recall(PID, LogRecallFilter::default()),
        Err(LogRecallError::CapabilityDenied)
    );
    let rows = world.operation_rows();
    assert_eq!(rows.len(), 2);
    assert_call(&rows[0], "fetch");
    assert_receipt(&rows[1], "recall", "capability-denied", None, None);
    assert_eq!(world.adapter_calls.load(Ordering::SeqCst), 1);
    world.assert_no_mint();
    clock.advance();
    assert_eq!(
        world
            .recall
            .recall(PID, LogRecallFilter::default())
            .unwrap()
            .entries[0]
            .frame_id,
        OWN
    );
    world.assert_issued_and_revoked(&world.operation_rows()[2], Scope::LogRecall);
}

/// Observe the real Researcher caller without replacing the mediated port's
/// mint, query, fetch, refusal, receipt or revocation behavior.
struct ObservedRecall {
    mediated: MediatedLogRecall,
    cursors: Mutex<Vec<Option<maos_domain::log_recall::LogRecallCursor>>>,
    fetched: Mutex<Vec<[u8; 16]>>,
}

impl LogRecallPort for ObservedRecall {
    fn recall(
        &self,
        pid: u32,
        filter: LogRecallFilter,
    ) -> Result<maos_domain::log_recall::LogRecallPage, LogRecallError> {
        self.cursors.lock().push(filter.cursor.clone());
        self.mediated.recall(pid, filter)
    }

    fn fetch(
        &self,
        pid: u32,
        id: [u8; 16],
    ) -> Result<maos_domain::log_recall::LogFetchResponse, LogRecallError> {
        let result = self.mediated.fetch(pid, id);
        if result.is_ok() {
            self.fetched.lock().push(id);
        }
        result
    }
}

struct Literature(Vec<researcher::FetchedClaim>);

impl researcher::ResearcherMcpPort for Literature {
    fn survey_literature(
        &self,
        _: &str,
    ) -> Result<Vec<researcher::FetchedClaim>, researcher::ResearcherMcpError> {
        Ok(self.0.clone())
    }
}

#[test]
fn researcher_catches_up_across_quota_windows_without_duplicate_or_tokenless_fetch() {
    use maos_spirit_sdk::{Ctx, Spirit};
    let clock = Arc::new(ManualClock::default());
    let mut world = quota_world(3, &clock);
    let ids: Vec<_> = (1..=7).map(|i| [i; 16]).collect();
    let claims = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let source_key = format!("paper-{i}");
            world.tl.insert_frame_event_with_id(
                Some(*id),
                FrameKind::McpInvocation,
                PID,
                "recall-test",
                "",
                None,
                "mcp:arxiv/get_paper",
                &serde_json::to_vec(&serde_json::json!({"arxiv_id": source_key})).unwrap(),
                FrameOrigin::Kernel,
            );
            researcher::FetchedClaim {
                source_key,
                claim: researcher::ClaimPayload {
                    claim_id: format!("claim-{i}"),
                    statement: format!("Finding {i}"),
                    topic: format!("topic-{i}"),
                    methodology_strength: 0.9,
                    confidence: 0.95,
                    load_bearing: true,
                    polarity: true,
                    hedges: vec![],
                },
            }
        })
        .collect();
    let observed = Arc::new(ObservedRecall {
        mediated: world.recall,
        cursors: Mutex::new(Vec::new()),
        fetched: Mutex::new(Vec::new()),
    });
    let researcher = researcher::Researcher::new()
        .with_mcp_port(Arc::new(Literature(claims)))
        .with_log_recall_port(Arc::clone(&observed) as _, Arc::new(AtomicU32::new(PID)));
    let mut previous_rows = 0;
    for window in 0..4 {
        researcher.on_idle(&mut Ctx::mock());
        let rows: Vec<_> = world
            .tl
            .query_frames(FrameFilter {
                order_by_insertion: true,
                ..Default::default()
            })
            .unwrap()
            .into_iter()
            .filter(|r| r.intent.starts_with("log."))
            .collect();
        let fresh = &rows[previous_rows..];
        let mut events = Vec::new();
        while let Ok(event) = world.audit.try_recv() {
            events.push(event);
        }
        let issued: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                CapAuditEvent::Issue {
                    token_id,
                    spirit_pid,
                    ..
                } => {
                    assert_eq!(*spirit_pid, PID);
                    Some(capability_token_column(token_id))
                }
                _ => None,
            })
            .collect();
        let calls: Vec<_> = fresh
            .iter()
            .filter(|r| r.kind == FrameKind::CapabilityInvocation)
            .collect();
        assert_eq!(
            calls.len(),
            issued.len(),
            "window {window}: one in-window mint per adapter call"
        );
        assert!(issued.len() <= 3);
        for (row, token) in calls.iter().zip(&issued) {
            assert_eq!(
                row.capability_token,
                Some(*token),
                "window {window}: never a tokenless or stale-token call"
            );
        }
        if window < 3 {
            assert!(researcher.last_output().is_none());
            assert_eq!(observed.fetched.lock().len(), (window + 1) * 2);
            assert_eq!(
                fresh
                    .iter()
                    .filter(|r| r.intent == "log.fetch.refused:capability-denied")
                    .count(),
                1
            );
        } else {
            let output = researcher
                .last_output()
                .expect("backlog completes in a later window");
            assert_eq!(output.findings.len(), ids.len());
            let references: std::collections::BTreeSet<_> = output
                .findings
                .iter()
                .map(|f| f.source_log_ref.clone())
                .collect();
            assert_eq!(
                references,
                ids.iter().map(researcher::encode_frame_id_hex).collect()
            );
        }
        assert!(!world.capability.has_active_tokens_for_pid(PID));
        previous_rows = rows.len();
        clock.advance();
    }
    assert_eq!(
        *observed.fetched.lock(),
        ids,
        "each backlog payload fetched exactly once"
    );
    let cursors = observed.cursors.lock();
    assert_eq!(cursors.len(), 4);
    assert_eq!(cursors[0], None);
    for (cursor, id) in cursors[1..].iter().zip([ids[1], ids[3], ids[5]]) {
        assert_eq!(
            cursor.as_ref().unwrap().last_frame_id,
            id,
            "resume after the last successful fetch"
        );
    }
    assert_eq!(world.adapter_calls.load(Ordering::SeqCst), 11);
}

#[tokio::test]
async fn quota_audit_and_mcp_receipts_are_operator_visible_but_never_spirit_disclosed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transparency.sqlite");
    let clock = Arc::new(ManualClock::default());
    let quota = small_quota(8, &clock);
    let mut world = World::with_tracker(
        vec![Scope::LogRecall, Scope::LogFetch],
        quota.clone(),
        Arc::new(TransparencyLogAdapter::open(&path, NONCE).unwrap()),
    );
    world.seed(OWN, PID, PAYLOAD);
    let reservations: Vec<_> = (0..8).map(|_| quota.charge(PID).unwrap()).collect();
    assert!(matches!(
        world.capability.issue_with_mediation(
            PID,
            Scope::LogRecall,
            60,
            [0; 32],
            IntentClass::Standard
        ),
        Err(maos_domain::ports::capability::CapError::IssuanceQuotaExhausted { spirit_id: PID })
    ));
    assert!(matches!(
        maos_bin::worker_spawn::issue_governed_mcp_capability(
            &world.capability,
            None,
            None,
            &world.tl,
            PID,
            "recall-test",
            "denied-server",
            "denied-tool",
            [0; 32],
        ),
        Err(maos_bin::worker_spawn::GovernedMintError::KernelPolicyDenied)
    ));
    // Forward the real queued events to the production writer and join it:
    // all rows are durable before either disclosure attempt.
    let (tx, rx) = cap_audit::channel();
    for event in world.drain_audit() {
        tx.try_send(event).unwrap();
    }
    drop(tx);
    cap_audit::CapAuditWriter::spawn(rx, Arc::clone(&world.tl))
        .await
        .unwrap();
    drop(reservations);
    clock.advance();
    let private: Vec<_> = world
        .tl
        .query_frames(FrameFilter::default())
        .unwrap()
        .into_iter()
        .filter(|r| {
            r.intent == "cap.quota:exhausted" || r.intent == "mcp.call.refused:capability-denied"
        })
        .collect();
    assert_eq!(private.len(), 2);
    let operator = maos_audit::query(
        &path,
        maos_audit::AuditFilter {
            spirit_pid: Some(PID),
            ..Default::default()
        },
    )
    .unwrap();
    for row in &private {
        assert!(operator
            .iter()
            .any(|entry| entry.frame_id_hex == hex::encode(row.frame_id)
                && entry.intent == row.intent));
    }
    let page = world
        .recall
        .recall(PID, LogRecallFilter::default())
        .unwrap();
    assert_eq!(
        page.entries.iter().map(|e| e.frame_id).collect::<Vec<_>>(),
        vec![OWN]
    );
    for row in private {
        assert!(
            matches!(world.recall.fetch(PID, row.frame_id), Err(LogRecallError::ScopeViolation { frame_id, .. }) if frame_id == row.frame_id),
            "private operator row must refuse without a payload"
        );
    }
}
