//! Journal-first admission mechanics. Prepare is side-effect free; the root
//! commits the returned approval row before inserting, materializing or saving.
use crate::discovery::DiscoveredSkill;
use crate::errors::ESkillQueue;
use crate::proposal::{build_proposal, SkillRevisionProposal};
use crate::schema::{parse_skill, Skill, SkillId};
use crate::store::{atomic_write, AuthorIdentity, ESkillStore, QueueEntry, SkillPayload};
use maos_domain::invariants::i4::ApprovalDecision;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkillAdmissionState {
    Pending,
    Admitted,
    Rejected,
}

/// Package discovery is the sole non-live entry path.
#[derive(Debug, Clone, PartialEq)]
pub enum SkillEntryPath {
    PackageShipped,
}
impl SkillEntryPath {
    pub fn label(&self) -> &'static str {
        "package_shipped"
    }
}

/// Trusted inputs resolved by the root at the author's fixed pid.
pub struct SubmissionContext<'a> {
    pub author: &'a AuthorIdentity,
    pub declared_actions: &'a BTreeSet<String>,
    pub principal_ids: &'a [String],
}

#[derive(Debug, Default)]
pub struct SkillAdmissionQueue {
    pending: Vec<QueueEntry>,
    audit: Vec<ApprovalDecision>,
}

impl SkillAdmissionQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_stored(entries: Vec<QueueEntry>) -> Result<Self, ESkillStore> {
        let mut ids = BTreeSet::new();
        for entry in &entries {
            Self::validate_entry(entry)?;
            if !ids.insert(&entry.entry_id) && !entry.legacy {
                return Err(ESkillQueue::InvalidStoredEntry("duplicate entry id".into()).into());
            }
        }
        Ok(Self {
            pending: entries,
            audit: Vec::new(),
        })
    }

    fn validate_entry(entry: &QueueEntry) -> Result<(), ESkillStore> {
        let bad = || ESkillQueue::InvalidStoredEntry(entry.entry_id.clone());
        if !matches!(
            entry.entry_path.as_str(),
            "package_shipped" | "author_self" | "revision_proposal"
        ) {
            return Err(ESkillQueue::UnknownEntryPath(entry.entry_path.clone()).into());
        }
        if entry.id.0.is_empty()
            || !entry
                .id
                .0
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-.".contains(&c))
            || semver::Version::parse(&entry.version.0).is_err()
        {
            return Err(bad().into());
        }
        let mut sealed = entry.clone();
        sealed.seal()?;
        if entry.entry_id != sealed.entry_id || entry.payload_sha256 != sealed.payload_sha256 {
            return Err(bad().into());
        }
        match (&entry.payload, &entry.author, entry.entry_path.as_str()) {
            (Some(SkillPayload::AuthorSelf(document)), Some(author), "author_self")
                if !entry.legacy =>
            {
                let skill = parse_skill(document).map_err(ESkillQueue::Schema)?;
                Self::validate_authored_id(&entry.id.0)?;
                if skill.manifest.skill_id() != entry.id
                    || skill.manifest.skill_version() != entry.version
                    || !entry.id.0.starts_with(&format!("{}.", author.class))
                {
                    return Err(bad().into());
                }
            }
            (Some(SkillPayload::RevisionProposal(p)), Some(author), "revision_proposal")
                if !entry.legacy =>
            {
                build_proposal(
                    p.target_skill_id.clone(),
                    p.target_version.clone(),
                    p.proposed_diff.clone(),
                    p.telemetry_evidence.clone(),
                )
                .map_err(ESkillQueue::Proposal)?;
                Self::validate_evidence(p)?;
                if p.target_skill_id != entry.id
                    || p.target_version != entry.version
                    || p.telemetry_evidence.spirit_pid != author.pid
                {
                    return Err(bad().into());
                }
            }
            (None, None, "package_shipped") => {}
            (None, None, _) if entry.legacy => {}
            _ => return Err(bad().into()),
        }
        if entry
            .author
            .as_ref()
            .is_some_and(|a| a.class.is_empty() || a.boot_nonce.is_empty())
        {
            return Err(bad().into());
        }
        Ok(())
    }

    /// `discovered` must contain the root's complete package discovery snapshot.
    pub fn prepare_author(
        &self,
        document: String,
        ctx: &SubmissionContext<'_>,
        discovered: &[DiscoveredSkill],
    ) -> Result<(QueueEntry, ApprovalDecision), ESkillStore> {
        Self::vet_principals(&document, ctx)?;
        let skill = parse_skill(&document).map_err(ESkillQueue::Schema)?;
        Self::validate_authored_id(&skill.manifest.id)?;
        if !skill
            .manifest
            .id
            .starts_with(&format!("{}.", ctx.author.class))
        {
            return Err(ESkillQueue::ForeignNamespace.into());
        }
        if skill
            .manifest
            .required_capabilities
            .iter()
            .any(|c| !ctx.declared_actions.contains(&c.0))
        {
            return Err(ESkillQueue::CapabilityEscalation.into());
        }
        let next = semver::Version::parse(&skill.manifest.version)
            .map_err(|_| ESkillQueue::StaleTargetVersion)?;
        let earlier = self
            .pending
            .iter()
            .filter(|e| e.id.0 == skill.manifest.id && e.entry_path != "revision_proposal")
            .map(|e| e.version.0.as_str())
            .chain(
                discovered
                    .iter()
                    .filter(|e| e.skill.manifest.id == skill.manifest.id)
                    .map(|e| e.skill.manifest.version.as_str()),
            );
        for version in earlier {
            let previous =
                semver::Version::parse(version).map_err(|_| ESkillQueue::StaleTargetVersion)?;
            if !next.cmp_precedence(&previous).is_gt() {
                return Err(ESkillQueue::StaleTargetVersion.into());
            }
        }
        self.prepare(
            skill.manifest.skill_id(),
            skill.manifest.skill_version(),
            "author_self",
            Some(ctx.author.clone()),
            Some(SkillPayload::AuthorSelf(document)),
        )
    }

    pub fn prepare_proposal(
        &self,
        proposal: SkillRevisionProposal,
        ctx: &SubmissionContext<'_>,
    ) -> Result<(QueueEntry, ApprovalDecision), ESkillStore> {
        Self::vet_principals(&proposal.proposed_diff, ctx)?;
        Self::validate_evidence(&proposal)?;
        let proposal = build_proposal(
            proposal.target_skill_id,
            proposal.target_version,
            proposal.proposed_diff,
            proposal.telemetry_evidence,
        )
        .map_err(ESkillQueue::Proposal)?;
        let current = self
            .current_owned(&proposal.target_skill_id, &ctx.author.class)
            .ok_or(ESkillQueue::TargetNotOwned)?;
        if current.version != proposal.target_version {
            return Err(ESkillQueue::StaleTargetVersion.into());
        }
        if proposal.telemetry_evidence.spirit_pid != ctx.author.pid {
            return Err(ESkillQueue::EvidenceMismatch.into());
        }
        self.prepare(
            proposal.target_skill_id.clone(),
            proposal.target_version.clone(),
            "revision_proposal",
            Some(ctx.author.clone()),
            Some(SkillPayload::RevisionProposal(proposal)),
        )
    }

    fn validate_authored_id(id: &str) -> Result<(), ESkillQueue> {
        if id.len() > 128 {
            return Err(ESkillQueue::AuthoredIdTooLong);
        }
        Ok(())
    }

    fn validate_evidence(proposal: &SkillRevisionProposal) -> Result<(), ESkillQueue> {
        if proposal
            .telemetry_evidence
            .halt_events
            .iter()
            .any(|e| !e.value.is_finite() || e.threshold.is_some_and(|v| !v.is_finite()))
        {
            return Err(ESkillQueue::NonFiniteEvidence);
        }
        Ok(())
    }

    fn vet_principals(text: &str, ctx: &SubmissionContext<'_>) -> Result<(), ESkillQueue> {
        if ctx
            .principal_ids
            .iter()
            .any(|id| maos_domain::memory::principal_id_occurs(text.as_bytes(), id))
        {
            return Err(ESkillQueue::PrincipalDataRefused);
        }
        Ok(())
    }

    fn current_owned(&self, id: &SkillId, class: &str) -> Option<&QueueEntry> {
        self.pending
            .iter()
            .filter(|e| {
                &e.id == id
                    && e.state == SkillAdmissionState::Admitted
                    && matches!(e.payload, Some(SkillPayload::AuthorSelf(_)))
                    && e.author.as_ref().is_some_and(|a| a.class == class)
            })
            .max_by_key(|e| semver::Version::parse(&e.version.0).ok())
    }

    fn prepare(
        &self,
        id: SkillId,
        version: crate::schema::SkillVersion,
        path: &str,
        author: Option<AuthorIdentity>,
        payload: Option<SkillPayload>,
    ) -> Result<(QueueEntry, ApprovalDecision), ESkillStore> {
        let mut entry = QueueEntry {
            id,
            version,
            entry_path: path.into(),
            state: SkillAdmissionState::Pending,
            entry_id: String::new(),
            author,
            payload,
            payload_sha256: String::new(),
            legacy: false,
        };
        entry.seal()?;
        self.check_insert(&entry)?;
        let actor = match &entry.author {
            Some(a) => format!("spirit:{}@{}", a.class, a.pid),
            None => "package".into(),
        };
        let row = Self::row(&entry, actor, "skill.admission.enqueue", false);
        Ok((entry, row))
    }

    fn check_insert(&self, entry: &QueueEntry) -> Result<(), ESkillQueue> {
        if self.pending.iter().any(|e| {
            e.entry_id == entry.entry_id
                || (e.id == entry.id
                    && e.state == SkillAdmissionState::Pending
                    && if entry.entry_path == "revision_proposal" {
                        e.entry_path == "revision_proposal" && e.author == entry.author
                    } else {
                        e.entry_path != "revision_proposal"
                    })
        }) {
            return Err(ESkillQueue::DuplicateSkillId(entry.id.to_string()));
        }
        Ok(())
    }

    /// Call only after the enqueue row commits. The root then saves to its store.
    pub fn commit_prepared(&mut self, entry: QueueEntry) -> Result<(), ESkillStore> {
        Self::validate_entry(&entry)?;
        if entry.state != SkillAdmissionState::Pending {
            return Err(ESkillQueue::InvalidStoredEntry(entry.entry_id).into());
        }
        self.check_insert(&entry)?;
        self.pending.push(entry);
        Ok(())
    }

    /// Package admission harness; dynamic submissions must use the vetted API.
    pub fn enqueue_skill(
        &mut self,
        skill: Skill,
        path: SkillEntryPath,
        actor: impl Into<String>,
    ) -> Result<SkillId, ESkillStore> {
        let (entry, mut row) = self.prepare(
            skill.manifest.skill_id(),
            skill.manifest.skill_version(),
            path.label(),
            None,
            None,
        )?;
        let id = entry.id.clone();
        row.actor = actor.into();
        self.commit_prepared(entry)?;
        self.audit.push(row);
        Ok(id)
    }

    pub fn show(&self, selector: &str) -> Result<&QueueEntry, ESkillQueue> {
        if let Some(e) = self.pending.iter().find(|e| e.entry_id == selector) {
            return Ok(e);
        }
        let mut matches = self.pending.iter().filter(|e| e.id.0 == selector);
        let entry = matches.next().ok_or(ESkillQueue::EntryNotFound)?;
        if matches.next().is_some() {
            return Err(ESkillQueue::AmbiguousEntry);
        }
        Ok(entry)
    }

    /// Preflight before journaling; authored approvals require their destination.
    pub fn prepare_decision(
        &self,
        selector: &str,
        admit: bool,
        actor: String,
        directory: Option<&Path>,
    ) -> Result<ApprovalDecision, ESkillStore> {
        let entry = self.show(selector)?;
        if entry.state != SkillAdmissionState::Pending {
            return Err(ESkillQueue::AlreadyDecided.into());
        }
        if admit && matches!(entry.payload, Some(SkillPayload::AuthorSelf(_))) {
            self.materialize_destination(entry, directory)?;
        }
        Ok(Self::row(
            entry,
            actor,
            if admit {
                "skill.admission.approve"
            } else {
                "skill.admission.reject"
            },
            admit,
        ))
    }

    /// Side-effect-free filesystem checks; repeat on apply to catch intervening changes.
    fn materialize_destination(
        &self,
        entry: &QueueEntry,
        directory: Option<&Path>,
    ) -> Result<PathBuf, ESkillStore> {
        Self::validate_authored_id(&entry.id.0)?;
        let directory = directory.ok_or(ESkillStore::NoDataHome)?;
        for ancestor in directory.ancestors().filter(|p| !p.as_os_str().is_empty()) {
            match std::fs::metadata(ancestor) {
                Ok(meta) if meta.is_dir() => break,
                Ok(_) => return Err(std::io::Error::from(std::io::ErrorKind::NotADirectory).into()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            }
        }
        let dest = directory.join(format!("{}.md", entry.id));
        match std::fs::read(&dest) {
            Ok(existing) => {
                let owned = self.pending.iter().any(|old| {
                    old.id == entry.id && old.state == SkillAdmissionState::Admitted
                        && old.author.as_ref().map(|a| &a.class) == entry.author.as_ref().map(|a| &a.class)
                        && matches!(&old.payload, Some(SkillPayload::AuthorSelf(text)) if text.as_bytes() == existing)
                });
                if !owned {
                    return Err(ESkillStore::MaterializeCollision(dest));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        Ok(dest)
    }

    /// After journaling, materialize an authored document and transition. The
    /// caller MUST save and propagate any write failure (the TL reconciles state).
    pub fn apply_decision(
        &mut self,
        selector: &str,
        admit: bool,
        directory: Option<&Path>,
    ) -> Result<(), ESkillStore> {
        let entry = self.show(selector)?;
        if entry.state != SkillAdmissionState::Pending {
            return Err(ESkillQueue::AlreadyDecided.into());
        }
        if admit {
            if let Some(SkillPayload::AuthorSelf(document)) = &entry.payload {
                let dest = self.materialize_destination(entry, directory)?;
                atomic_write(&dest, document.as_bytes())?;
            }
        }
        let key = entry.entry_id.clone();
        if let Some(entry) = self.pending.iter_mut().find(|e| e.entry_id == key) {
            entry.state = if admit {
                SkillAdmissionState::Admitted
            } else {
                SkillAdmissionState::Rejected
            };
        }
        Ok(())
    }

    fn row(
        entry: &QueueEntry,
        actor: String,
        capability: &str,
        decision: bool,
    ) -> ApprovalDecision {
        ApprovalDecision {
            actor,
            target: entry.approval_target(),
            capability: capability.into(),
            intent: entry.entry_path.clone(),
            decision,
            reasoning: Some(format!(
                "entry_id={} payload_sha256={}",
                entry.entry_id, entry.payload_sha256
            )),
        }
    }
    pub fn own_entries(&self, class: &str) -> Vec<&QueueEntry> {
        self.pending
            .iter()
            .filter(|e| e.author.as_ref().is_some_and(|a| a.class == class))
            .collect()
    }
    pub fn entries(&self) -> &[QueueEntry] {
        &self.pending
    }
    pub fn to_stored(&self) -> Vec<QueueEntry> {
        self.pending.clone()
    }
    pub fn state_of(&self, id: &SkillId) -> Option<SkillAdmissionState> {
        self.pending.iter().find(|e| &e.id == id).map(|e| e.state)
    }
    pub fn pending_count(&self) -> usize {
        self.pending
            .iter()
            .filter(|e| e.state == SkillAdmissionState::Pending)
            .count()
    }
    pub fn audit_trail(&self) -> &[ApprovalDecision] {
        &self.audit
    }
}
