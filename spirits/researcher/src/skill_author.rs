//! Native authoring boundary: identity and evidence are fixed by the host.

use crate::{LiveInference, Researcher, ResearcherError, ResearcherInferenceBinding, SurveyOutput};
use maos_domain::invariants::i1::CapabilityToken;
use maos_skill::{QueueEntry, SkillAdmissionState, SkillPayload};

/// A successful submission is still pending operator admission.
#[derive(Debug, Clone, PartialEq)]
pub enum SkillAuthorOutcome {
    Authored(QueueEntry),
    Proposed(QueueEntry),
    /// The owned draft/proposal already awaits an operator decision.
    PendingExisting(QueueEntry),
}

/// Stable model prompt shared by the live pass and strict replay witnesses.
/// Only model findings enter the prompt, never boot-local frame ids or pids.
/// `base` is an admitted, owned `AuthorSelf` entry returned by the host.
pub fn authoring_prompt(output: &SurveyOutput, base: Option<&QueueEntry>) -> String {
    let mut prompt = match base {
        Some(base) => {
            let mut prompt = format!(
                "Propose one improvement to the admitted skill {}@{}. \
                 Return only a non-empty unified diff against the document below, \
                 without code fences or commentary. Do not change its id, version, \
                 or required capabilities. The host attaches telemetry evidence; \
                 do not invent evidence.\n\nAdmitted document:\n",
                base.id, base.version
            );
            if let Some(SkillPayload::AuthorSelf(document)) = &base.payload {
                prompt.push_str(document);
            }
            prompt
        }
        None => String::from(
            "Author one reusable research skill from the survey findings below. \
             Return only a maos.skill.v1 Markdown document, without code fences or \
             commentary. Open with --- TOML frontmatter ---, containing id = \
             \"researcher.<name>\" (choose a descriptive lowercase name), version = \
             \"0.1.0\", name, description, and required_capabilities = []. \
             Follow it with a non-empty Markdown procedure. Preserve uncertainty; \
             do not invent facts or telemetry evidence.",
        ),
    };
    prompt.push_str("\n\nSurvey findings:\n");
    for finding in &output.findings {
        prompt.push_str("- ");
        prompt.push_str(&finding.statement);
        prompt.push('\n');
    }
    prompt
}

impl Researcher {
    pub(crate) fn author_after_survey(
        &self,
        live: &LiveInference,
        binding: &ResearcherInferenceBinding,
        token: &CapabilityToken,
        pid: u32,
        output: &SurveyOutput,
    ) -> Result<(), ResearcherError> {
        let Some(port) = &self.skill_author_port else {
            return Ok(());
        };
        let entries = port.own_entries().map_err(ResearcherError::SkillAuthor)?;
        let base = entries
            .iter()
            .filter(|entry| {
                entry.state == SkillAdmissionState::Admitted
                    && matches!(&entry.payload, Some(SkillPayload::AuthorSelf(_)))
            })
            .min_by(|left, right| {
                (&left.id, &left.version, &left.entry_id).cmp(&(
                    &right.id,
                    &right.version,
                    &right.entry_id,
                ))
            });
        let pending = entries.iter().find(|entry| {
            entry.state == SkillAdmissionState::Pending
                && match (base, &entry.payload) {
                    (None, Some(SkillPayload::AuthorSelf(_))) => true,
                    (Some(base), Some(SkillPayload::RevisionProposal(_))) => {
                        entry.id == base.id && entry.version == base.version
                    }
                    _ => false,
                }
        });
        if let Some(entry) = pending {
            *self
                .skill_author_outcome
                .lock()
                .unwrap_or_else(|error| error.into_inner()) =
                Some(SkillAuthorOutcome::PendingExisting(entry.clone()));
            return Ok(());
        }
        let text = self.complete_bound(
            live,
            binding,
            token,
            pid,
            authoring_prompt(output, base),
            2048,
        )?;
        let outcome = match base {
            Some(base) => port
                .propose(base.id.0.clone(), base.version.0.clone(), text)
                .map(SkillAuthorOutcome::Proposed),
            None => port.author(text).map(SkillAuthorOutcome::Authored),
        }
        .map_err(ResearcherError::SkillAuthor)?;
        *self
            .skill_author_outcome
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(outcome);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SkillPortError {
    #[error("host-shut-down")]
    HostShutDown,
    #[error("capability-denied")]
    CapabilityDenied,
    #[error("context-exhausted")]
    ContextExhausted,
    #[error("quota-exhausted")]
    QuotaExhausted,
    #[error("submission refused: {0}")]
    Submission(#[from] maos_skill::ESkillStore),
    #[error("journal failed: {0}")]
    Journal(String),
    #[error("capability mediation failed: {0}")]
    Mediation(String),
}

pub trait SkillAuthorPort: Send + Sync {
    fn author(&self, document: String) -> Result<maos_skill::QueueEntry, SkillPortError>;
    fn propose(
        &self,
        target_id: String,
        target_version: String,
        diff: String,
    ) -> Result<maos_skill::QueueEntry, SkillPortError>;
    fn own_entries(&self) -> Result<Vec<maos_skill::QueueEntry>, SkillPortError>;
}
