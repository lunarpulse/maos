//! Operator-admission queue (FR39) — the kernel-mediated gate every skill
//! crosses before activation.
//!
//! Mirrors the `OrchestratorInstruction` ordered-pending-set + Approval-
//! Decision-Log-row-on-enqueue pattern (`maos-kernel-core/src/orchestrator`).
//! NOTHING enters `Admitted` without an operator `approve` call — the
//! capability `skill.author.self` (Story 7.4) authorizes the WRITE-to-queue,
//! NOT the activation.
//!
//! The queue holds an in-process audit trail of [`ApprovalDecision`] rows
//! (the canonical shape the Transparency Log / Approval Decision Log records).
//! Decoupling the queue from the journal adapter — exactly as
//! `OrchestratorBuffer` is decoupled from `journal_orchestrator_queue` — keeps
//! `maos-skill` pure (no `maos-iac` dependency); the kernel composition root
//! drains [`SkillAdmissionQueue::audit_trail`] into the real journal port.

use maos_domain::invariants::i4::ApprovalDecision;

use crate::errors::ESkillQueue;
use crate::proposal::SkillRevisionProposal;
use crate::schema::{Skill, SkillId, SkillVersion};
use crate::approval_target::approval_target;

/// Admission state of a queued skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkillAdmissionState {
    /// Awaiting operator admission (FR39). The default landing state for ALL
    /// three entry paths — no skill is ever born `Admitted`.
    Pending,
    /// Operator-admitted — the skill may now activate.
    Admitted,
    /// Operator-rejected — the skill will not activate.
    Rejected,
}

/// The three FR39 entry paths into the admission queue, distinguishable in
/// audit.
#[derive(Debug, Clone, PartialEq)]
pub enum SkillEntryPath {
    /// Shipped inside a Spirit package.
    PackageShipped,
    /// Written dynamically at runtime via the `skill.author.self` capability.
    AuthorSelf,
    /// An FR57 revision proposal built from a Spirit's own self-telemetry.
    RevisionProposal(SkillRevisionProposal),
}

impl SkillEntryPath {
    /// A stable audit label distinguishing the entry path.
    pub fn label(&self) -> &'static str {
        match self {
            SkillEntryPath::PackageShipped => "package_shipped",
            SkillEntryPath::AuthorSelf => "author_self",
            SkillEntryPath::RevisionProposal(_) => "revision_proposal",
        }
    }

    /// Reconstruct from a stored label.  `RevisionProposal` entries lose their
    /// proposal payload on deserialization (the proposal content is not stored;
    /// state transitions depend only on `id` + `state`, not the payload).
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "package_shipped" => Some(SkillEntryPath::PackageShipped),
            "author_self" => Some(SkillEntryPath::AuthorSelf),
            // RevisionProposal can't be reconstructed (proposal payload not
            // persisted).  The caller falls back to PackageShipped, which
            // preserves the state-transition semantics (approve/reject depend
            // on id+state only).  The authoritative entry_path label is in
            // the QueueEntry.entry_path field, not here.
            _ => None,
        }
    }
}

/// One queued entry: a new skill (`PackageShipped` / `AuthorSelf`, carrying the
/// full [`Skill`]) OR an FR57 revision proposal (carrying the proposal; `skill`
/// is `None` because a revision targets an EXISTING skill).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingEntry {
    /// Skill id — `manifest.id` for a new skill; `target_skill_id` for a revision.
    pub id: SkillId,
    /// Skill version — `manifest.version` for a new skill; `target_version` for a revision.
    pub version: SkillVersion,
    /// Which path the entry took into the queue.
    pub entry_path: SkillEntryPath,
    /// The full skill (new-skill paths) or `None` (revision proposals).
    pub skill: Option<Skill>,
    /// Current admission state.
    pub state: SkillAdmissionState,
}

/// An ordered operator-admission queue with an in-process audit trail.
#[derive(Debug, Default)]
pub struct SkillAdmissionQueue {
    pending: Vec<PendingEntry>,
    audit: Vec<ApprovalDecision>,
}

impl SkillAdmissionQueue {
    /// A fresh, empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a NEW skill via the `PackageShipped` or `AuthorSelf` path.
    ///
    /// Lands the skill `Pending` (FR39 — never auto-admitted) and appends an
    /// Approval-Decision-Log row recording the admission REQUEST (decision =
    /// `false`: not yet admitted) with the entry-path label in `intent`.
    ///
    /// # Panics
    /// Never. A `RevisionProposal` entry_path passed here is recorded as-is,
    /// but callers should prefer [`Self::enqueue_proposal`] for proposals.
    pub fn enqueue_skill(
        &mut self,
        skill: Skill,
        entry_path: SkillEntryPath,
        actor: impl Into<String>,
    ) -> Result<SkillId, ESkillQueue> {
        let id = skill.manifest.skill_id();
        let version = skill.manifest.skill_version();
        if self
            .pending
            .iter()
            .any(|e| &e.id == &id && e.state == SkillAdmissionState::Pending)
        {
            return Err(ESkillQueue::DuplicateSkillId(id.to_string()));
        }
        self.audit.push(ApprovalDecision {
            actor: actor.into(),
            target: approval_target(&id, &version),
            capability: "skill.admission.enqueue".into(),
            intent: entry_path.label().into(),
            decision: false, // pending — operator has not yet admitted
            reasoning: Some(format!(
                "skill {}@{} entered admission queue via {}",
                id,
                version,
                entry_path.label()
            )),
        });
        self.pending.push(PendingEntry {
            id: id.clone(),
            version,
            entry_path,
            skill: Some(skill),
            state: SkillAdmissionState::Pending,
        });
        Ok(id)
    }

    /// Enqueue an FR57 revision proposal.
    ///
    /// Lands `Pending` with an Approval-Decision-Log row distinguishable in
    /// audit as a `revision_proposal` (carrying telemetry evidence), subject to
    /// the SAME operator approve/reject + audit obligations as a new skill.
    pub fn enqueue_proposal(
        &mut self,
        proposal: SkillRevisionProposal,
        actor: impl Into<String>,
    ) -> Result<SkillId, ESkillQueue> {
        let id = proposal.target_skill_id.clone();
        let version = proposal.target_version.clone();
        if self
            .pending
            .iter()
            .any(|e| &e.id == &id && e.state == SkillAdmissionState::Pending)
        {
            return Err(ESkillQueue::DuplicateSkillId(id.to_string()));
        }
        let evidence_pid = proposal.telemetry_evidence.spirit_pid;
        self.audit.push(ApprovalDecision {
            actor: actor.into(),
            target: approval_target(&id, &version),
            capability: "skill.admission.enqueue".into(),
            intent: "revision_proposal".into(),
            decision: false, // pending — operator has not yet admitted
            reasoning: Some(format!(
                "skill-revision proposal for {}@{} entered admission queue (telemetry_evidence from spirit_pid={}, diff_len={})",
                id,
                version,
                evidence_pid,
                proposal.proposed_diff.len()
            )),
        });
        self.pending.push(PendingEntry {
            id: id.clone(),
            version,
            entry_path: SkillEntryPath::RevisionProposal(proposal),
            skill: None,
            state: SkillAdmissionState::Pending,
        });
        Ok(id)
    }

    /// Operator-admit the first `Pending` entry with the given id. Transitions
    /// it to `Admitted` and journals the operator decision (decision = `true`).
    /// Returns `true` if an entry transitioned.
    pub fn approve(&mut self, id: &SkillId) -> bool {
        self.transition(id, SkillAdmissionState::Admitted, "skill.admission.approve")
    }

    /// Operator-reject the first `Pending` entry with the given id. Transitions
    /// it to `Rejected` and journals the operator decision (decision = `false`).
    /// Returns `true` if an entry transitioned.
    pub fn reject(&mut self, id: &SkillId) -> bool {
        self.transition(id, SkillAdmissionState::Rejected, "skill.admission.reject")
    }

    fn transition(&mut self, id: &SkillId, to: SkillAdmissionState, capability: &str) -> bool {
        if let Some(entry) = self
            .pending
            .iter_mut()
            .find(|e| &e.id == id && e.state == SkillAdmissionState::Pending)
        {
            entry.state = to;
            let admitted = to == SkillAdmissionState::Admitted;
            let target = approval_target(&entry.id, &entry.version);
            let path_label = entry.entry_path.label();
            self.audit.push(ApprovalDecision {
                actor: "operator".into(),
                target,
                capability: capability.into(),
                intent: path_label.into(),
                decision: admitted,
                reasoning: Some(format!(
                    "operator {} {} ({})",
                    if admitted { "admitted" } else { "rejected" },
                    id,
                    path_label
                )),
            });
            true
        } else {
            false
        }
    }

    /// All currently-queued entries, in insertion order.
    pub fn entries(&self) -> &[PendingEntry] {
        &self.pending
    }

    /// The admission state of the first entry with the given id, if any.
    pub fn state_of(&self, id: &SkillId) -> Option<SkillAdmissionState> {
        self.pending.iter().find(|e| &e.id == id).map(|e| e.state)
    }

    /// Count of entries in the `Pending` state.
    pub fn pending_count(&self) -> usize {
        self.pending
            .iter()
            .filter(|e| e.state == SkillAdmissionState::Pending)
            .count()
    }

    /// The in-process Approval-Decision-Log audit trail. The kernel composition
    /// root drains this into the real journal port (the same shape
    /// `TransparencyLogAdapter::insert_approval_decision` records).
    pub fn audit_trail(&self) -> &[ApprovalDecision] {
        &self.audit
    }
}

// ---------------------------------------------------------------------------
// Store integration (Story 9.7 AC-1)
// ---------------------------------------------------------------------------

impl SkillAdmissionQueue {
    /// Reconstruct the queue from durable [`QueueEntry`] records loaded by the
    /// [`SkillQueueStore`].
    ///
    /// The `skill` field on each entry is `None` (skill content is discovered
    /// from the filesystem, not persisted in the store).  The in-memory audit
    /// trail starts empty — cross-restart audit history is served by
    /// `query_approvals()` against the Transparency Log.
    ///
    /// [`QueueEntry`]: crate::store::QueueEntry
    /// [`SkillQueueStore`]: crate::store::SkillQueueStore
    pub fn from_stored(entries: Vec<crate::store::QueueEntry>) -> Self {
        let pending = entries
            .into_iter()
            .map(|e| PendingEntry {
                id: e.id,
                version: e.version,
                entry_path: SkillEntryPath::from_label(&e.entry_path)
                    .unwrap_or(SkillEntryPath::PackageShipped),
                skill: None,
                state: e.state,
            })
            .collect();
        Self {
            pending,
            audit: Vec::new(),
        }
    }

    /// Export the current queue state to durable [`QueueEntry`] records for
    /// the [`SkillQueueStore`].
    ///
    /// [`QueueEntry`]: crate::store::QueueEntry
    /// [`SkillQueueStore`]: crate::store::SkillQueueStore
    pub fn to_stored(&self) -> Vec<crate::store::QueueEntry> {
        self.pending
            .iter()
            .map(|e| crate::store::QueueEntry {
                id: e.id.clone(),
                version: e.version.clone(),
                entry_path: e.entry_path.label().to_string(),
                state: e.state,
            })
            .collect()
    }
}
