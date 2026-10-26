#![forbid(unsafe_code)]
#![cfg(feature = "network")]

//! Story 18-7 AC2: the production native port over real capability, security,
//! policy, telemetry, SCB, transparency-log and durable queue adapters.
//! Every fixture uses explicit scratch paths, never process-global HOME edits.
//! No daemon, model, network, sleeps, or compiler subprocesses are needed.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use maos_bin::skill_authoring::LiveSkillAuthorPort;
use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::{MemoryManagerPort, SelfTelemetryPort};
use maos_kernel_core::capability::cap_audit::CapAuditWriter;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::{CapQuotaTracker, IssuanceQuota};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::context_budget::ContextWindow;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::iac::transparency_log::{
    FrameFilter, FrameKind, TransparencyLogAdapter, TransparencyLogEntry,
};
use maos_kernel_core::memory::self_telemetry::SelfTelemetryAggregator;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, SpiritControlBlock, SpiritManifestBundle,
};
use maos_kernel_core::security::{RingCryptoProvider, SecurityManagerAdapter};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_skill::{
    ESkillProposal, ESkillQueue, ESkillSchema, ESkillStore, LocalFsSkillQueueStore, QueueEntry,
    SkillAdmissionQueue, SkillAdmissionState, SkillPayload, SkillQueueStore,
};
use researcher::{Researcher, SkillAuthorPort, SkillPortError};

const PID: u32 = 187;
const OTHER_PID: u32 = 9187;
const BOOT: u64 = 0x18_07_01;
const CLASS: &str = "researcher";
const ID: &str = "researcher.root-witness";

struct Root {
    home: tempfile::TempDir,
    capability: Arc<CapabilityRegistryAdapter>,
    policy: Arc<PolicyTable>,
    security: Arc<SecurityManagerAdapter>,
    journal: Arc<TransparencyLogAdapter>,
    telemetry: Arc<SelfTelemetryAggregator>,
    scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    store: Arc<LocalFsSkillQueueStore>,
    serialization: Arc<Mutex<()>>,
    runtime: tokio::runtime::Runtime,
    writer: tokio::task::JoinHandle<()>,
}

/// Field order closes SQLite before removing its scratch directory.
struct DrainedRoot {
    journal: Arc<TransparencyLogAdapter>,
    _home: tempfile::TempDir,
}
impl std::ops::Deref for DrainedRoot {
    type Target = TransparencyLogAdapter;
    fn deref(&self) -> &Self::Target {
        &self.journal
    }
}

impl Root {
    fn new(declared: bool, denied: bool, quota: u64) -> Self {
        init_monotonic_base();
        let home = tempfile::tempdir().unwrap();
        let journal = Arc::new(
            TransparencyLogAdapter::open(&home.path().join("transparency.sqlite"), BOOT).unwrap(),
        );
        PrincipalNamespaceIndex::open(&home.path().join("transparency.sqlite")).unwrap();
        let policy = Arc::new(PolicyTable::new());
        let mut snapshot = PolicyTableInner::default();
        for pid in [PID, OTHER_PID] {
            snapshot.manifest_scopes.insert(
                pid,
                ManifestCapabilityScope {
                    scopes: if declared {
                        vec![Scope::SkillAuthorSelf]
                    } else {
                        vec![]
                    },
                    declared_tier: SandboxTier(0),
                    trust_tier: TrustTier::Verified,
                },
            );
        }
        if denied {
            snapshot
                .operator_policy
                .per_capability_deny
                .insert("skill.author.self".into());
        }
        policy.update(snapshot);
        let security = Arc::new(SecurityManagerAdapter::new(policy.clone()));
        let (tx, rx) = maos_kernel_core::capability::cap_audit::channel();
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(RingCryptoProvider),
            Ed25519SigningKey::new([0x18; 32]),
            BOOT,
            policy.clone(),
            tx,
            CapQuotaTracker::with_quota(IssuanceQuota {
                budget_per_window: quota,
                window: Duration::from_secs(3600),
            }),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        ));
        let telemetry = Arc::new(SelfTelemetryAggregator::new(
            Arc::new(IacRtMetrics::new()),
            Arc::new(HaltRegistry::new()),
            journal.clone(),
        ));
        let scbs = Arc::new(RwLock::new(BTreeMap::from([
            (
                PID,
                Arc::new(SpiritControlBlock::new(
                    PID,
                    CLASS.into(),
                    SpiritManifestBundle::default(),
                    make_spirit_obj(Researcher::new()),
                    BOOT,
                )),
            ),
            (
                OTHER_PID,
                Arc::new(SpiritControlBlock::new(
                    OTHER_PID,
                    "butler".into(),
                    SpiritManifestBundle::default(),
                    make_spirit_obj(Researcher::new()),
                    BOOT,
                )),
            ),
        ])));
        let store = Arc::new(LocalFsSkillQueueStore::at_path(
            home.path().join("queue.json"),
        ));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let writer = {
            let _entered = runtime.enter();
            CapAuditWriter::spawn(rx, journal.clone())
        };
        Self {
            home,
            capability,
            policy,
            security,
            journal,
            telemetry,
            scbs,
            store,
            serialization: maos_bin::skill_authoring::serialization(),
            runtime,
            writer,
        }
    }

    fn port(&self, pid: u32, since_ns: u64) -> LiveSkillAuthorPort {
        LiveSkillAuthorPort {
            pid: Arc::new(move || Some(pid)),
            capability: Arc::downgrade(&self.capability),
            security: Arc::downgrade(&self.security),
            journal: Arc::downgrade(&self.journal),
            telemetry: Arc::downgrade(&self.telemetry),
            scbs: Arc::downgrade(&self.scbs),
            store: Arc::downgrade(&self.store),
            serialization: self.serialization.clone(),
            enterprise: None,
            pdp: None,
            since_ns,
            declared_actions: ["skill.author.self".to_owned()].into_iter().collect(),
        }
    }

    fn entries(&self) -> Vec<QueueEntry> {
        self.store.load().unwrap()
    }

    fn receipt(&self, code: &str, count: usize) {
        let rows = self
            .journal
            .query_frames(FrameFilter {
                intent: Some(format!("skill.author.refused:{code}")),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            rows.len(),
            count,
            "one non-personal receipt per refused call"
        );
        for row in rows {
            assert_eq!(row.spirit_pid, PID);
            assert_eq!(row.boot_nonce, BOOT);
            assert_eq!(row.kind, FrameKind::CapabilityInvocation);
            let payload: serde_json::Value = serde_json::from_slice(&row.payload_redacted).unwrap();
            assert_eq!(
                payload,
                serde_json::json!({"spirit_pid": PID, "reason": code})
            );
        }
    }

    fn admit(&self, entry: &QueueEntry) {
        let mut queue = SkillAdmissionQueue::from_stored(self.entries()).unwrap();
        let row = queue
            .prepare_decision(&entry.entry_id, true, "operator".into())
            .unwrap();
        self.journal.insert_approval_decision(row).unwrap();
        queue
            .apply_decision(&entry.entry_id, true, Some(self.home.path()))
            .unwrap();
        self.store.save(&queue.to_stored()).unwrap();
    }

    fn index_principal(&self, pid: u32, principal: &str) {
        let db = self.home.path().join("transparency.sqlite");
        let memory = MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(
                self.home.path().join("memory"),
                4096,
            )),
            Arc::new(SharedMemoryStore::open(&db).unwrap()),
            Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
            self.journal.clone(),
        );
        memory
            .write(
                pid,
                MemoryTier::Private,
                &MemoryNamespace::Principal {
                    principal_id: principal.into(),
                    schema: "skill-port-18-7".into(),
                },
                "record",
                MemoryValue::Blob(b"private record".to_vec()),
            )
            .unwrap();
        assert!(self
            .journal
            .principal_ids_for_spirit_pid(pid)
            .unwrap()
            .contains(&principal.into()));
    }

    /// Keep the actual port alive while dropping every root-owned sender.
    /// The real audit writer must observe channel closure, not time out.
    fn drain(self) -> DrainedRoot {
        let Self {
            home,
            capability,
            policy,
            security,
            journal,
            telemetry,
            scbs,
            store,
            serialization,
            runtime,
            writer,
        } = self;
        drop((
            capability,
            policy,
            security,
            telemetry,
            scbs,
            store,
            serialization,
        ));
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(2), writer)
                .await
                .expect("weak authoring port must not pin an audit sender")
                .expect("production audit writer must finish cleanly");
        });
        DrainedRoot {
            journal,
            _home: home,
        }
    }
}

fn document(id: &str, body: &str) -> String {
    format!("---\nid='{id}'\nversion='1.0.0'\nname='Root witness'\ndescription='Research procedure'\nrequired_capabilities=['skill.author.self']\n---\n{body}\n")
}
fn valid() -> String {
    document(
        ID,
        "Compare the available sources without inventing certainty.",
    )
}
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
fn frames(log: &TransparencyLogAdapter) -> Vec<TransparencyLogEntry> {
    log.query_frames(FrameFilter::default()).unwrap()
}
fn submission_error(result: Result<QueueEntry, SkillPortError>, expected: ESkillQueue) {
    assert!(
        matches!(result, Err(SkillPortError::Submission(ESkillStore::InvalidEntry(error)))
        if error == expected)
    );
}
fn no_enqueue(root: &Root) {
    assert!(root.entries().is_empty());
    assert!(root.journal.query_approvals(None).unwrap().is_empty());
}

#[test]
fn port_trait_accepts_only_content_not_pid_class_evidence_or_decision() {
    // These method-item coercions fail compilation if any caller-controlled
    // identity or evidence argument is added to the Spirit-facing boundary.
    let _: fn(&LiveSkillAuthorPort, String) -> Result<QueueEntry, SkillPortError> =
        <LiveSkillAuthorPort as SkillAuthorPort>::author;
    let _: fn(&LiveSkillAuthorPort, String, String, String) -> Result<QueueEntry, SkillPortError> =
        <LiveSkillAuthorPort as SkillAuthorPort>::propose;
    let _: fn(&LiveSkillAuthorPort) -> Result<Vec<QueueEntry>, SkillPortError> =
        <LiveSkillAuthorPort as SkillAuthorPort>::own_entries;
}

#[test]
fn author_uses_root_pid_and_scb_class_mints_verifies_and_persists_enqueue_payload() {
    let root = Root::new(true, false, 100);
    let port = root.port(PID, 0);
    let text = valid();
    let entry = port.author(text.clone()).unwrap();
    let author = entry.author.as_ref().unwrap();
    assert_eq!((author.pid, author.class.as_str()), (PID, CLASS));
    assert_eq!(author.boot_nonce, BOOT.to_string());
    assert_eq!(entry.state, SkillAdmissionState::Pending);
    assert_eq!(entry.payload, Some(SkillPayload::AuthorSelf(text.clone())));
    let mut resealed = entry.clone();
    resealed.seal().unwrap();
    assert_eq!(entry, resealed);
    assert_eq!(root.entries(), vec![entry.clone()]);
    assert_eq!(port.own_entries().unwrap(), vec![entry.clone()]);
    assert!(root.port(OTHER_PID, 0).own_entries().unwrap().is_empty());
    let rows = root.journal.query_approvals(None).unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.intent, "author_self");
    assert_eq!(row.actor, format!("spirit:{CLASS}@{PID}"));
    assert_eq!(row.target, format!("{ID}@1.0.0"));
    assert_eq!(row.capability, "skill.admission.enqueue");
    let reasoning = row.reasoning.as_ref().unwrap();
    assert!(reasoning.contains(&entry.entry_id));
    assert!(reasoning.contains(&entry.payload_sha256));
    assert!(!reasoning.contains("Compare the available sources"));
    let log = root.drain();
    let rows = frames(&log);
    let issues: Vec<_> = rows
        .iter()
        .filter(|r| r.intent.starts_with("cap.issue."))
        .collect();
    let verified: Vec<_> = rows
        .iter()
        .filter(|r| r.intent == "cap.verify.ok")
        .collect();
    assert_eq!((issues.len(), verified.len()), (1, 1));
    assert_eq!(
        serde_json::from_slice::<Scope>(&issues[0].payload_redacted).unwrap(),
        Scope::SkillAuthorSelf
    );
    assert_eq!(issues[0].spirit_pid, PID);
    assert_eq!(verified[0].spirit_pid, PID);
    assert_eq!(issues[0].origin, FrameOrigin::Kernel);
    assert_eq!(verified[0].origin, FrameOrigin::Kernel);
    assert!(issues[0].capability_token.is_some());
    assert_eq!(issues[0].capability_token, verified[0].capability_token);
    assert!(issues[0].insertion_id < verified[0].insertion_id);
}

#[test]
fn undeclared_author_and_proposal_are_typed_journaled_and_never_enqueued() {
    let root = Root::new(false, false, 100);
    let port = root.port(PID, 0);
    assert!(matches!(
        port.author(valid()),
        Err(SkillPortError::CapabilityDenied)
    ));
    assert!(matches!(
        port.propose(ID.into(), "1.0.0".into(), "+change".into()),
        Err(SkillPortError::CapabilityDenied)
    ));
    root.receipt("capability-denied", 2);
    no_enqueue(&root);
    assert!(!frames(&root.drain())
        .iter()
        .any(|r| r.intent.starts_with("cap.issue.")));
}

#[test]
fn org_deny_overrides_declared_authority_for_author_and_proposal() {
    let root = Root::new(true, true, 100);
    let port = root.port(PID, 0);
    assert!(matches!(
        port.author(valid()),
        Err(SkillPortError::CapabilityDenied)
    ));
    assert!(matches!(
        port.propose(ID.into(), "1.0.0".into(), "+change".into()),
        Err(SkillPortError::CapabilityDenied)
    ));
    root.receipt("capability-denied", 2);
    no_enqueue(&root);
    assert!(!frames(&root.drain())
        .iter()
        .any(|r| r.intent.starts_with("cap.issue.")));
}

#[test]
fn exhausted_context_refuses_both_submissions_at_root_fixed_pid() {
    let root = Root::new(true, false, 100);
    root.capability.context_budget().report(
        PID,
        ContextWindow {
            size: 100,
            pressure_permille: 800,
            limit_permille: 950,
        },
        101,
    );
    let port = root.port(PID, 0);
    assert!(matches!(
        port.author(valid()),
        Err(SkillPortError::ContextExhausted)
    ));
    assert!(matches!(
        port.propose(ID.into(), "1.0.0".into(), "+change".into()),
        Err(SkillPortError::ContextExhausted)
    ));
    root.receipt("context-exhausted", 2);
    no_enqueue(&root);
    assert!(!root.capability.context_budget().exhausted(OTHER_PID));
    assert!(!frames(&root.drain())
        .iter()
        .any(|r| r.intent.starts_with("cap.issue.")));
}

#[test]
fn issuance_quota_exhaustion_is_typed_journaled_and_preserves_existing_queue() {
    let root = Root::new(true, false, 1);
    let port = root.port(PID, 0);
    let first = port.author(valid()).unwrap();
    assert!(matches!(
        port.author(document("researcher.second", "Another procedure")),
        Err(SkillPortError::QuotaExhausted)
    ));
    assert!(matches!(
        port.propose(ID.into(), "1.0.0".into(), "+change".into()),
        Err(SkillPortError::QuotaExhausted)
    ));
    root.receipt("quota-exhausted", 2);
    assert_eq!(root.entries(), vec![first]);
    assert_eq!(root.journal.query_approvals(None).unwrap().len(), 1);
    let rows = frames(&root.drain());
    assert_eq!(
        rows.iter()
            .filter(|r| r.intent.starts_with("cap.issue."))
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|r| r.intent == "cap.quota:exhausted" && r.spirit_pid == PID)
            .count(),
        1
    );
}

#[test]
fn library_ownership_schema_escalation_and_principal_refusals_reach_typed_root_receipts() {
    let root = Root::new(true, false, 100);
    let port = root.port(PID, 0);
    submission_error(
        port.author(document("butler.foreign", "Foreign body")),
        ESkillQueue::ForeignNamespace,
    );
    root.receipt("foreign-namespace", 1);
    submission_error(
        port.author(valid().replace("skill.author.self", "memory.write")),
        ESkillQueue::CapabilityEscalation,
    );
    root.receipt("capability-escalation", 1);
    submission_error(
        port.author("malformed document".into()),
        ESkillQueue::Schema(ESkillSchema::MissingFence),
    );
    root.receipt("skill-schema", 1);
    submission_error(
        port.propose("butler.foreign".into(), "1.0.0".into(), "+foreign".into()),
        ESkillQueue::TargetNotOwned,
    );
    root.receipt("target-not-owned", 1);
    no_enqueue(&root);
    root.index_principal(PID, "private-person-187");
    submission_error(
        port.author(document(ID, "Do not disclose private-person-187.")),
        ESkillQueue::PrincipalDataRefused,
    );
    root.receipt("principal-data-refused", 1);
    no_enqueue(&root);
    let foreign_port = root.port(OTHER_PID, 0);
    let foreign = foreign_port
        .author(document("butler.foreign", "Foreign procedure"))
        .unwrap();
    assert_eq!(foreign.author.as_ref().unwrap().pid, OTHER_PID);
    assert_eq!(foreign.author.as_ref().unwrap().class, "butler");
    root.admit(&foreign);
    let foreign_before = root.entries();
    let foreign_rows = root.journal.query_approvals(None).unwrap();
    submission_error(
        port.propose("butler.foreign".into(), "1.0.0".into(), "+overwrite".into()),
        ESkillQueue::TargetNotOwned,
    );
    root.receipt("target-not-owned", 2);
    assert_eq!(root.entries(), foreign_before);
    assert_eq!(root.journal.query_approvals(None).unwrap(), foreign_rows);
    assert!(port.own_entries().unwrap().is_empty());
    assert_eq!(foreign_port.own_entries().unwrap().len(), 1);
    let base = port.author(valid()).unwrap();
    root.admit(&base);
    let before = root.entries();
    let approvals = root.journal.query_approvals(None).unwrap();
    submission_error(
        port.propose(ID.into(), "0.9.0".into(), "+stale".into()),
        ESkillQueue::StaleTargetVersion,
    );
    root.receipt("stale-target-version", 1);
    submission_error(
        port.propose(ID.into(), "1.0.0".into(), " \n".into()),
        ESkillQueue::Proposal(ESkillProposal::EmptyDiff),
    );
    root.receipt("skill-proposal", 1);
    submission_error(
        port.propose(ID.into(), "1.0.0".into(), "+private-person-187".into()),
        ESkillQueue::PrincipalDataRefused,
    );
    root.receipt("principal-data-refused", 2);
    assert_eq!(root.entries(), before);
    assert_eq!(root.journal.query_approvals(None).unwrap(), approvals);
    let rows = frames(&root.drain());
    for row in rows
        .iter()
        .filter(|r| r.intent.starts_with("skill.author.refused:"))
    {
        assert!(!format!("{row:?}").contains("private-person-187"));
    }
    assert_eq!(
        rows.iter()
            .filter(|r| r.intent.starts_with("skill.author.refused:"))
            .count(),
        9
    );
}

#[test]
fn proposal_evidence_is_root_telemetry_for_bound_pid_and_bind_window() {
    let root = Root::new(true, false, 100);
    let base_port = root.port(PID, 0);
    let base = base_port.author(valid()).unwrap();
    root.admit(&base);
    root.journal.insert_frame_event(
        FrameKind::TaskComplete,
        PID,
        None,
        "before-bind",
        b"",
        FrameOrigin::SpiritAuto,
    );
    // Bind immediately after an actual old row, without a sleep or clock tick
    // assumption. A later root-port call cannot supply its own pid or window.
    let old = frames(&root.journal)
        .into_iter()
        .find(|r| r.intent == "before-bind")
        .unwrap();
    let since = old.timestamp_ns + 1;
    let port = root.port(PID, since);
    for (pid, kind, intent) in [
        (PID, FrameKind::TaskComplete, "own-success"),
        (PID, FrameKind::EpistemicHalt, "own-failure"),
        (OTHER_PID, FrameKind::TaskComplete, "foreign-success"),
    ] {
        root.journal
            .insert_frame_event(kind, pid, None, intent, b"", FrameOrigin::SpiritAuto);
    }
    let expected = root.telemetry.self_telemetry(PID, Some(since)).unwrap();
    assert_eq!((expected.success_count, expected.failure_count), (1, 1));
    let started = now_ns();
    let diff = "--- old\n+++ new\n@@\n-old\n+consider contrary evidence\n";
    let entry = port
        .propose(ID.into(), "1.0.0".into(), diff.into())
        .unwrap();
    let finished = now_ns();
    let Some(SkillPayload::RevisionProposal(proposal)) = &entry.payload else {
        panic!("full proposal payload required")
    };
    assert_eq!(proposal.target_skill_id.0, ID);
    assert_eq!(proposal.target_version.0, "1.0.0");
    assert_eq!(proposal.proposed_diff, diff);
    let evidence = &proposal.telemetry_evidence;
    assert_eq!(
        (evidence.spirit_pid, evidence.window_start_ns),
        (PID, since)
    );
    assert!((started..=finished).contains(&evidence.window_end_ns));
    assert!((evidence.window_end_ns..=finished).contains(&evidence.generated_ns));
    // The real aggregator samples wall time on EACH call. Compare every other
    // field verbatim and prove the stored samples against its exact audit row.
    let mut comparable = evidence.clone();
    comparable.window_end_ns = expected.window_end_ns;
    comparable.generated_ns = expected.generated_ns;
    assert_eq!(comparable, expected);
    let reads = root
        .journal
        .query_frames(FrameFilter {
            intent: Some("telemetry.self".into()),
            spirit_pid: Some(PID),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(reads.len(), 2);
    assert_eq!(
        reads[1].payload_redacted,
        format!(
            "self_telemetry: pid={PID} window=[{since},{}]",
            evidence.window_end_ns
        )
        .as_bytes()
    );
    let stored = root.entries();
    assert_eq!(
        stored.iter().find(|e| e.entry_id == entry.entry_id),
        Some(&entry)
    );
    let rows = root.journal.query_approvals(None).unwrap();
    let row = rows
        .iter()
        .find(|r| r.target == format!("{ID}@1.0.0#{}", entry.entry_id))
        .unwrap();
    assert_eq!(row.intent, "revision_proposal");
    assert_eq!(row.capability, "skill.admission.enqueue");
    assert!(row
        .reasoning
        .as_ref()
        .unwrap()
        .contains(&entry.payload_sha256));
    assert_eq!(entry.author.as_ref().unwrap().pid, PID);
    let log = root.drain();
    assert_eq!(
        frames(&log)
            .iter()
            .filter(|r| r.intent == "cap.verify.ok")
            .count(),
        2
    );
}

#[test]
fn weak_services_shutdown_returns_host_shutdown_for_every_method_and_drains_audit() {
    let root = Root::new(true, false, 100);
    let port = root.port(PID, 0);
    port.author(valid()).unwrap();
    let capability = Arc::downgrade(&root.capability);
    let security = Arc::downgrade(&root.security);
    let telemetry = Arc::downgrade(&root.telemetry);
    let scbs = Arc::downgrade(&root.scbs);
    let store = Arc::downgrade(&root.store);
    let journal = Arc::downgrade(&root.journal);
    assert_eq!(Arc::strong_count(&root.capability), 1);
    assert_eq!(Arc::strong_count(&root.security), 1);
    assert_eq!(Arc::strong_count(&root.telemetry), 1);
    assert_eq!(Arc::strong_count(&root.scbs), 1);
    assert_eq!(Arc::strong_count(&root.store), 1);
    let log = root.drain();
    assert!(capability.upgrade().is_none());
    assert!(security.upgrade().is_none());
    assert!(telemetry.upgrade().is_none());
    assert!(scbs.upgrade().is_none());
    assert!(store.upgrade().is_none());
    assert!(matches!(
        port.author(valid()),
        Err(SkillPortError::HostShutDown)
    ));
    assert!(matches!(
        port.propose(ID.into(), "1.0.0".into(), "+change".into()),
        Err(SkillPortError::HostShutDown)
    ));
    assert!(matches!(
        port.own_entries(),
        Err(SkillPortError::HostShutDown)
    ));
    assert_eq!(
        frames(&log)
            .iter()
            .filter(|r| r.intent == "cap.verify.ok")
            .count(),
        1
    );
    drop(log);
    assert!(
        journal.upgrade().is_none(),
        "port must not keep the root journal alive"
    );
}
