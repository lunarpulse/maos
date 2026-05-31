//! FR57 skill-revision proposals.
//!
//! A Spirit reads its OWN performance telemetry (the EXISTING Story 4.3
//! `SelfTelemetryPort` / `SelfTelemetryReport`, FR56) and emits a proposal
//! carrying the three FR57-mandated payload fields. The proposal enters the
//! SAME operator-admission queue as a new skill, under the SAME vetting/audit
//! obligations. Story 7.4 CONSUMES FR56 — it adds NO telemetry plumbing.

use maos_domain::self_telemetry::SelfTelemetryReport;

use crate::errors::ESkillProposal;
use crate::schema::{SkillId, SkillVersion};

/// An FR57 skill-revision proposal — the three mandated payload fields plus
/// the telemetry evidence supporting it.
///
/// `proposed_diff` is OPAQUE unified-diff text; the kernel does NOT interpret
/// it (§4.0.7). `telemetry_evidence` is the EXISTING Story 4.3 report shape,
/// carried verbatim (the known v0.3-β limitations — latency quantiles may be
/// `(0,0,0)`, principal-namespace filtering best-effort — are INHERITED).
#[derive(Debug, Clone, PartialEq)]
pub struct SkillRevisionProposal {
    /// (a) the target skill id.
    pub target_skill_id: SkillId,
    /// (a) the target skill version.
    pub target_version: SkillVersion,
    /// (b) the proposed diff — opaque unified-diff text.
    pub proposed_diff: String,
    /// (c) the telemetry evidence supporting the proposal.
    pub telemetry_evidence: SelfTelemetryReport,
}

/// Build a well-formed [`SkillRevisionProposal`].
///
/// Validates: target id non-empty, target version valid semver, diff non-empty.
/// The telemetry evidence is a required field (its presence is guaranteed by
/// the type). Story 7.4 does NOT add telemetry counters — the report is
/// obtained by the Spirit via the EXISTING `SelfTelemetryPort` and consumed
/// verbatim here.
pub fn build_proposal(
    target_skill_id: SkillId,
    target_version: SkillVersion,
    proposed_diff: String,
    telemetry_evidence: SelfTelemetryReport,
) -> Result<SkillRevisionProposal, ESkillProposal> {
    if target_skill_id.0.trim().is_empty() {
        return Err(ESkillProposal::EmptyTargetId);
    }
    if !target_skill_id
        .0
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
    {
        return Err(ESkillProposal::InvalidTargetIdCharset(
            target_skill_id.0.clone(),
        ));
    }
    semver::Version::parse(&target_version.0).map_err(|e| {
        ESkillProposal::InvalidTargetVersion(target_version.0.clone(), e.to_string())
    })?;
    if proposed_diff.trim().is_empty() {
        return Err(ESkillProposal::EmptyDiff);
    }
    Ok(SkillRevisionProposal {
        target_skill_id,
        target_version,
        proposed_diff,
        telemetry_evidence,
    })
}
