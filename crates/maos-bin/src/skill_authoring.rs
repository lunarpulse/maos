//! Root-owned native authoring: weak buses, fixed identity, journal before save.
use crate::enterprise_identity::EnterpriseRuntime;
use crate::enterprise_pdp_runtime::EnterprisePdpRuntime;
use crate::worker_spawn::{issue_enterprise_governed_capability, GovernedMintError};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::{scope_action_key, SecurityManagerPort, SelfTelemetryPort};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::self_telemetry::SelfTelemetryAggregator;
use maos_kernel_core::scheduler::control_block::SpiritControlBlock;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_skill::{
    AuthorIdentity, LocalFsSkillQueueStore, QueueEntry, SkillAdmissionQueue, SkillQueueStore,
    SubmissionContext,
};
use researcher::{PidSource, SkillAuthorPort, SkillPortError};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, RwLock, Weak};

type Scbs = RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>;

pub fn serialization() -> Arc<Mutex<()>> {
    static LOCK: std::sync::LazyLock<Arc<Mutex<()>>> =
        std::sync::LazyLock::new(|| Arc::new(Mutex::new(())));
    Arc::clone(&LOCK)
}

pub type SkillBuilder =
    Arc<dyn Fn(PidSource, &[Scope]) -> Result<Arc<dyn SkillAuthorPort>, String> + Send + Sync>;

#[allow(clippy::too_many_arguments)]
pub fn binder(
    capability: &Arc<CapabilityRegistryAdapter>,
    security: &Arc<SecurityManagerAdapter>,
    journal: &Arc<TransparencyLogAdapter>,
    telemetry: &Arc<SelfTelemetryAggregator>,
    scbs: &Arc<Scbs>,
    store: &Arc<LocalFsSkillQueueStore>,
    enterprise: Option<&Arc<EnterpriseRuntime>>,
    pdp: Option<EnterprisePdpRuntime>,
) -> SkillBuilder {
    let (capability, security, journal, telemetry, scbs, store) = (
        Arc::downgrade(capability),
        Arc::downgrade(security),
        Arc::downgrade(journal),
        Arc::downgrade(telemetry),
        Arc::downgrade(scbs),
        Arc::downgrade(store),
    );
    let enterprise = enterprise.map(Arc::downgrade);
    Arc::new(move |pid, scopes| {
        let since_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
            .min(u64::MAX as u128) as u64;
        Ok(Arc::new(LiveSkillAuthorPort {
            pid,
            capability: capability.clone(),
            security: security.clone(),
            journal: journal.clone(),
            telemetry: telemetry.clone(),
            scbs: scbs.clone(),
            store: store.clone(),
            serialization: serialization(),
            enterprise: enterprise.clone(),
            pdp: pdp.clone(),
            since_ns,
            declared_actions: scopes
                .iter()
                .map(|scope| scope_action_key(scope).to_owned())
                .collect(),
        }))
    })
}

pub struct LiveSkillAuthorPort {
    pub pid: PidSource,
    pub capability: Weak<CapabilityRegistryAdapter>,
    pub security: Weak<SecurityManagerAdapter>,
    pub journal: Weak<TransparencyLogAdapter>,
    pub telemetry: Weak<SelfTelemetryAggregator>,
    pub scbs: Weak<Scbs>,
    pub store: Weak<LocalFsSkillQueueStore>,
    pub serialization: Arc<Mutex<()>>,
    pub enterprise: Option<Weak<EnterpriseRuntime>>,
    pub pdp: Option<EnterprisePdpRuntime>,
    pub since_ns: u64,
    pub declared_actions: BTreeSet<String>,
}

fn upgrade<T>(value: &Weak<T>) -> Result<Arc<T>, SkillPortError> {
    value.upgrade().ok_or(SkillPortError::HostShutDown)
}

impl LiveSkillAuthorPort {
    fn submit(
        &self,
        document: Option<String>,
        proposal: Option<(String, String, String)>,
    ) -> Result<QueueEntry, SkillPortError> {
        let journal = upgrade(&self.journal)?;
        let capability = upgrade(&self.capability)?;
        let security = upgrade(&self.security)?;
        let telemetry = upgrade(&self.telemetry)?;
        let scbs = upgrade(&self.scbs)?;
        let store = upgrade(&self.store)?;
        let pid = (self.pid)().ok_or(SkillPortError::HostShutDown)?;
        let scb = scbs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&pid)
            .cloned()
            .ok_or(SkillPortError::HostShutDown)?;
        let result = (|| {
            let enterprise = self.enterprise.as_ref().map(upgrade).transpose()?;
            let token = issue_enterprise_governed_capability(
                &capability,
                enterprise.as_deref(),
                self.pdp.as_ref(),
                pid,
                Scope::SkillAuthorSelf,
                300,
                [0; 32],
                IntentClass::Standard,
            )
            .map_err(|error| match error {
                GovernedMintError::ContextExhausted => SkillPortError::ContextExhausted,
                GovernedMintError::QuotaExhausted => SkillPortError::QuotaExhausted,
                GovernedMintError::KernelPolicyDenied
                | GovernedMintError::PdpDenied(_)
                | GovernedMintError::SsoAssertionMissing(_) => SkillPortError::CapabilityDenied,
                error => SkillPortError::Mediation(error.to_string()),
            })?;
            let tier = security
                .effective_sandbox_tier(pid)
                .ok_or(SkillPortError::CapabilityDenied)?;
            capability
                .verify_and_audit(&token, [0; 32], tier)
                .map_err(|_| SkillPortError::CapabilityDenied)?;
            let principals = journal
                .principal_ids_for_spirit_pid(pid)
                .map_err(|error| SkillPortError::Journal(error.to_string()))?;
            let author = AuthorIdentity {
                class: scb.spirit_id.clone(),
                pid,
                boot_nonce: scb.boot_nonce.to_string(),
            };
            let context = SubmissionContext {
                author: &author,
                declared_actions: &self.declared_actions,
                principal_ids: &principals,
            };
            let _guard = self.serialization.lock().unwrap_or_else(|e| e.into_inner());
            let rows = journal
                .query_approvals(None)
                .map_err(|error| SkillPortError::Journal(error.to_string()))?;
            let stored = maos_cli::subcommands::reconcile_entries(
                store.load()?,
                &maos_cli::subcommands::decided_set(&rows),
            );
            let mut queue = SkillAdmissionQueue::from_stored(stored)?;
            let (entry, row) = match (document, proposal) {
                (Some(document), None) => queue.prepare_author(document, &context)?,
                (None, Some((id, version, diff))) => {
                    let evidence = telemetry
                        .self_telemetry(pid, Some(self.since_ns))
                        .map_err(|error| SkillPortError::Mediation(error.to_string()))?;
                    let proposal = maos_skill::build_proposal(
                        maos_skill::SkillId(id),
                        maos_skill::SkillVersion(version),
                        diff,
                        evidence,
                    )
                    .map_err(|error| {
                        maos_skill::ESkillStore::InvalidEntry(maos_skill::ESkillQueue::Proposal(
                            error,
                        ))
                    })?;
                    queue.prepare_proposal(proposal, &context)?
                }
                _ => return Err(SkillPortError::Mediation("invalid host submission".into())),
            };
            journal
                .insert_approval_decision(row)
                .map_err(|error| SkillPortError::Journal(error.to_string()))?;
            queue.commit_prepared(entry.clone())?;
            store.save(&queue.to_stored())?;
            Ok(entry)
        })();
        if let Err(error) = &result {
            let code = match error {
                SkillPortError::HostShutDown => "host-shut-down",
                SkillPortError::CapabilityDenied => "capability-denied",
                SkillPortError::ContextExhausted => "context-exhausted",
                SkillPortError::QuotaExhausted => "quota-exhausted",
                SkillPortError::Submission(maos_skill::ESkillStore::InvalidEntry(error)) => {
                    error.code()
                }
                SkillPortError::Submission(_) => "queue-unavailable",
                SkillPortError::Journal(_) => "journal-failed",
                SkillPortError::Mediation(_) => "mediation-failed",
            };
            let receipt =
                serde_json::to_vec(&serde_json::json!({"spirit_pid": pid, "reason": code}))
                    .map_err(|error| SkillPortError::Journal(error.to_string()))?;
            let _ = journal.insert_frame_event(
                FrameKind::CapabilityInvocation,
                pid,
                None,
                &format!("skill.author.refused:{code}"),
                &receipt,
                FrameOrigin::SpiritAuto,
            );
        }
        result
    }
}

impl SkillAuthorPort for LiveSkillAuthorPort {
    fn author(&self, document: String) -> Result<QueueEntry, SkillPortError> {
        self.submit(Some(document), None)
    }
    fn propose(
        &self,
        id: String,
        version: String,
        diff: String,
    ) -> Result<QueueEntry, SkillPortError> {
        self.submit(None, Some((id, version, diff)))
    }
    fn own_entries(&self) -> Result<Vec<QueueEntry>, SkillPortError> {
        let _capability = upgrade(&self.capability)?;
        let _security = upgrade(&self.security)?;
        let _telemetry = upgrade(&self.telemetry)?;
        let scbs = upgrade(&self.scbs)?;
        let store = upgrade(&self.store)?;
        let journal = upgrade(&self.journal)?;
        let pid = (self.pid)().ok_or(SkillPortError::HostShutDown)?;
        let class = scbs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&pid)
            .map(|scb| scb.spirit_id.clone())
            .ok_or(SkillPortError::HostShutDown)?;
        let _guard = self.serialization.lock().unwrap_or_else(|e| e.into_inner());
        let rows = journal
            .query_approvals(None)
            .map_err(|error| SkillPortError::Journal(error.to_string()))?;
        let stored = maos_cli::subcommands::reconcile_entries(
            store.load()?,
            &maos_cli::subcommands::decided_set(&rows),
        );
        let queue = SkillAdmissionQueue::from_stored(stored)?;
        Ok(queue.own_entries(&class).into_iter().cloned().collect())
    }
}
