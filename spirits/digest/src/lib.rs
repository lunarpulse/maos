#![forbid(unsafe_code)]

//! Read-only J3 team Digest Spirit.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use maos_cohort::{
    CohortDigestDistributor, CohortManifestState, DigestSummary, DIGEST_DAILY_SCOPE,
};
use maos_spirit_abi::identity::HostId;
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// Digest's cognitive posture. The manifest maps it to cautious autonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DigestPosture {
    PassiveObserver,
}

/// One consented member summary reconciled with receipt-presence evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentedMemberSummary {
    pub member: String,
    pub request_id: String,
    pub summary: DigestSummary,
    pub present_receipts: usize,
}

/// Read-only cohort digest composer. Network requests remain composition-owned;
/// this Spirit holds only the consented results exposed by `CohortManifestState`.
#[derive(Clone, Default)]
pub struct DigestSpirit {
    cohort_state: Option<Arc<CohortManifestState>>,
    pending_reads: Arc<Mutex<BTreeMap<String, String>>>,
    last_summaries: Arc<Mutex<Vec<ConsentedMemberSummary>>>,
}

#[spirit]
impl DigestSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        if !ctx.cancellation().is_cancelled() {
            self.refresh_consented_summaries();
        }
    }
}

impl DigestSpirit {
    pub fn with_cohort_state(state: Arc<CohortManifestState>) -> Self {
        Self {
            cohort_state: Some(state),
            ..Self::default()
        }
    }

    pub fn posture(&self) -> DigestPosture {
        DigestPosture::PassiveObserver
    }

    pub fn can_halt(&self) -> bool {
        false
    }

    pub fn can_arbitrate(&self) -> bool {
        false
    }

    /// Issue the real consent-gated `cohort:digest-read` request and retain its
    /// correlation id for a later `on_idle` collection pass.
    pub async fn request_consented_summary(
        &self,
        distributor: &CohortDigestDistributor,
        target: &HostId,
    ) -> Result<String, maos_cohort::CohortError> {
        let request_id = distributor.request_read(target, DIGEST_DAILY_SCOPE).await?;
        self.pending_reads
            .lock()
            .map_err(|_| maos_cohort::CohortError::EStatePoisoned)?
            .insert(target.as_str().to_string(), request_id.clone());
        Ok(request_id)
    }

    /// Reconcile already-admitted, request-id-deduplicated summaries with the
    /// receipt-presence stream. It never reads peer-private raw frames.
    pub fn refresh_consented_summaries(&self) {
        let Some(state) = &self.cohort_state else {
            return;
        };
        let Ok(pending) = self.pending_reads.lock() else {
            return;
        };
        let mut collected = Vec::with_capacity(pending.len());
        for (member, request_id) in pending.iter() {
            let host = HostId(member.clone());
            if let Some(summary) = state.digest_summary(&host, request_id) {
                collected.push(ConsentedMemberSummary {
                    member: member.clone(),
                    request_id: request_id.clone(),
                    summary,
                    present_receipts: state.present_receipt_count(&host),
                });
            }
        }
        if let Ok(mut last) = self.last_summaries.lock() {
            *last = collected;
        }
    }

    pub fn consented_summaries(&self) -> Vec<ConsentedMemberSummary> {
        self.last_summaries
            .lock()
            .map(|summaries| summaries.clone())
            .unwrap_or_default()
    }
}

/// A consented member self-report plus the Digest Spirit's own ingestion frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryEvidence {
    pub member: String,
    pub request_id: String,
    pub summary: DigestSummary,
    pub source_log_ref: String,
}

/// Receipt-presence evidence captured from the 12.3 stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HaltReceiptEvidence {
    pub member: String,
    pub halt_id: String,
    pub architectural_conflict: bool,
    pub source_log_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentOutcome {
    Resolved,
    Refused,
}

/// One consent-journal fact, projected from the Digest Spirit's own journaled
/// ingestion frame rather than a peer transparency log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentJournalEvidence {
    pub consultation_id: String,
    pub outcome: ConsentOutcome,
    pub source_log_ref: String,
}

impl ConsentJournalEvidence {
    pub fn resolved(consultation_id: impl Into<String>, source_log_ref: impl Into<String>) -> Self {
        Self {
            consultation_id: consultation_id.into(),
            outcome: ConsentOutcome::Resolved,
            source_log_ref: source_log_ref.into(),
        }
    }

    pub fn refused(consultation_id: impl Into<String>, source_log_ref: impl Into<String>) -> Self {
        Self {
            consultation_id: consultation_id.into(),
            outcome: ConsentOutcome::Refused,
            source_log_ref: source_log_ref.into(),
        }
    }
}

/// Raw, captured inputs. No pre-rendered narrative is accepted by this shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawDigestInputs {
    pub summaries: Vec<SummaryEvidence>,
    pub receipt_presence: Vec<HaltReceiptEvidence>,
    pub consent_journal: Vec<ConsentJournalEvidence>,
}

/// Derived J3 narrative and its clause-to-source I11 map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamDigest {
    pub agents_ran: usize,
    pub frames_exchanged: u64,
    pub agents_halted: usize,
    pub acted_invisibly: u64,
    pub consultations_resolved: usize,
    pub consultations_refused: usize,
    pub architectural_conflicts: usize,
    pub narrative: String,
    pub clause_sources: BTreeMap<String, Vec<String>>,
}

#[derive(Debug)]
pub enum DigestError {
    ConflictingEvidence(String),
    InvalidSourceLogRef(String),
    Serialization(String),
    Distillation(maos_domain::distillation::DistillationError),
}

impl std::fmt::Display for DigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictingEvidence(reason) => write!(f, "conflicting digest evidence: {reason}"),
            Self::InvalidSourceLogRef(value) => write!(f, "invalid source_log_ref: {value}"),
            Self::Serialization(reason) => write!(f, "digest serialization failed: {reason}"),
            Self::Distillation(error) => write!(f, "distillation failed: {error}"),
        }
    }
}

impl std::error::Error for DigestError {}

/// The single narrative derivation. The mesh capture supplies raw inputs only;
/// the daemon render and drift gate are its two production consumers.
pub fn derive_team_digest(raw: &RawDigestInputs) -> Result<TeamDigest, DigestError> {
    let mut summaries: BTreeMap<(&str, &str), &SummaryEvidence> = BTreeMap::new();
    for evidence in &raw.summaries {
        let key = (evidence.member.as_str(), evidence.request_id.as_str());
        if let Some(existing) = summaries.get(&key) {
            if existing.summary != evidence.summary {
                return Err(DigestError::ConflictingEvidence(format!(
                    "request {} for {} changed summary",
                    evidence.request_id, evidence.member
                )));
            }
            continue;
        }
        summaries.insert(key, evidence);
    }

    let mut receipts: BTreeMap<&str, &HaltReceiptEvidence> = BTreeMap::new();
    for evidence in &raw.receipt_presence {
        if let Some(existing) = receipts.get(evidence.halt_id.as_str()) {
            if existing.member != evidence.member
                || existing.architectural_conflict != evidence.architectural_conflict
            {
                return Err(DigestError::ConflictingEvidence(format!(
                    "halt receipt {} changed identity or classification",
                    evidence.halt_id
                )));
            }
            continue;
        }
        receipts.insert(evidence.halt_id.as_str(), evidence);
    }

    let mut consultations: BTreeMap<&str, &ConsentJournalEvidence> = BTreeMap::new();
    for evidence in &raw.consent_journal {
        if let Some(existing) = consultations.get(evidence.consultation_id.as_str()) {
            if existing.outcome != evidence.outcome {
                return Err(DigestError::ConflictingEvidence(format!(
                    "consultation {} changed outcome",
                    evidence.consultation_id
                )));
            }
            continue;
        }
        consultations.insert(evidence.consultation_id.as_str(), evidence);
    }

    // Collapse the (member, request_id)-keyed evidence to one entry per member.
    // The daily-digest invariant is one summary per member; a member read under
    // several request_ids (e.g. a replay under a fresh correlation id) must
    // report identical daily stats, otherwise the evidence conflicts. Counting
    // and visibility reconcile at member granularity so a replayed summary can
    // neither double-count frames nor double-subtract receipt presence.
    let mut per_member: BTreeMap<&str, &SummaryEvidence> = BTreeMap::new();
    for evidence in summaries.values() {
        match per_member.get(evidence.member.as_str()) {
            Some(existing) if existing.summary != evidence.summary => {
                return Err(DigestError::ConflictingEvidence(format!(
                    "member {} reported differing summaries across request_ids",
                    evidence.member
                )));
            }
            None => {
                per_member.insert(evidence.member.as_str(), evidence);
            }
            _ => {}
        }
    }

    let agents_ran = per_member.len();
    let frames_exchanged = per_member
        .values()
        .map(|evidence| evidence.summary.frames)
        .sum();
    let agents_halted = receipts
        .values()
        .map(|evidence| evidence.member.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let acted_invisibly = per_member
        .values()
        .map(|summary| {
            let present_receipts = receipts
                .values()
                .filter(|receipt| receipt.member == summary.member)
                .count() as u64;
            summary.summary.halts.saturating_sub(present_receipts)
        })
        .sum();
    let consultations_resolved = consultations
        .values()
        .filter(|evidence| evidence.outcome == ConsentOutcome::Resolved)
        .count();
    let consultations_refused = consultations.len() - consultations_resolved;
    let architectural_conflicts = receipts
        .values()
        .filter(|evidence| evidence.architectural_conflict)
        .count();

    let summary_refs = summaries
        .values()
        .map(|evidence| evidence.source_log_ref.clone())
        .collect::<Vec<_>>();
    let receipt_refs = receipts
        .values()
        .map(|evidence| evidence.source_log_ref.clone())
        .collect::<Vec<_>>();
    let consultation_refs = consultations
        .values()
        .map(|evidence| evidence.source_log_ref.clone())
        .collect::<Vec<_>>();
    let conflict_refs = receipts
        .values()
        .filter(|evidence| evidence.architectural_conflict)
        .map(|evidence| evidence.source_log_ref.clone())
        .collect::<Vec<_>>();
    let mut halt_visibility_refs = summary_refs.clone();
    halt_visibility_refs.extend(receipt_refs.iter().cloned());
    let clause_sources = BTreeMap::from([
        ("agents_ran".to_string(), summary_refs.clone()),
        ("frames_exchanged".to_string(), summary_refs),
        ("halts_and_visibility".to_string(), halt_visibility_refs),
        ("consultations".to_string(), consultation_refs),
        ("architectural_conflicts".to_string(), conflict_refs),
    ]);

    let agents_word = if agents_ran == 1 { "agent" } else { "agents" };
    let frame_word = if frames_exchanged == 1 {
        "frame"
    } else {
        "frames"
    };
    let halted_word = if agents_halted == 1 {
        "agent"
    } else {
        "agents"
    };
    let consultation_word = if consultations_resolved == 1 {
        "consultation"
    } else {
        "consultations"
    };
    let (refused_noun, refused_verb) = if consultations_refused == 1 {
        ("request", "was")
    } else {
        ("requests", "were")
    };
    let conflict_word = if architectural_conflicts == 1 {
        "conflict"
    } else {
        "conflicts"
    };
    let narrative = format!(
        "Overnight, {agents_ran} {agents_word} ran. {frames_exchanged} IAC {frame_word} exchanged. \
         {agents_halted} {halted_word} halted, {acted_invisibly} acted invisibly. \
         {consultations_resolved} cross-agent {consultation_word} resolved without escalation. \
         {consultations_refused} consent {refused_noun} {refused_verb} refused without exposing private evidence. \
         {architectural_conflicts} architectural {conflict_word} surfaced for review."
    );

    Ok(TeamDigest {
        agents_ran,
        frames_exchanged,
        agents_halted,
        acted_invisibly,
        consultations_resolved,
        consultations_refused,
        architectural_conflicts,
        narrative,
        clause_sources,
    })
}

fn decode_frame_id_hex(value: &str) -> Result<[u8; 16], DigestError> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DigestError::InvalidSourceLogRef(value.to_string()));
    }
    let mut frame_id = [0u8; 16];
    for (index, byte) in frame_id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| DigestError::InvalidSourceLogRef(value.to_string()))?;
    }
    Ok(frame_id)
}

/// Butler-shaped persist tail: structured payload, one-hop depth, and the
/// deduplicated union of clause-owned evidence refs.
pub fn digest_to_distillation_request(
    digest: &TeamDigest,
) -> Result<maos_domain::distillation::DistillationRequest, DigestError> {
    let mut source_log_ref = std::collections::BTreeSet::new();
    for value in digest.clause_sources.values().flatten() {
        source_log_ref.insert(decode_frame_id_hex(value)?);
    }
    let payload = maos_domain::distillation::DigestPayload::Json(
        serde_json::to_value(digest)
            .map_err(|error| DigestError::Serialization(error.to_string()))?,
    );
    maos_domain::distillation::DistillationRequest::new(
        source_log_ref.into_iter().collect(),
        1,
        payload,
        None,
    )
    .map_err(DigestError::Distillation)
}

pub fn persist_team_digest(
    writer: &dyn maos_domain::ports::DistillationPort,
    spirit_pid: u32,
    digest: &TeamDigest,
) -> Result<maos_domain::distillation::DistillationReceipt, DigestError> {
    let request = digest_to_distillation_request(digest)?;
    writer
        .write_distillate(spirit_pid, request)
        .map_err(DigestError::Distillation)
}
#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_sdk::Spirit;

    fn assert_spirit<T: Spirit>() {}

    #[test]
    fn digest_is_a_passive_observer_spirit() {
        assert_spirit::<DigestSpirit>();
        let digest = DigestSpirit::default();
        assert_eq!(digest.posture(), DigestPosture::PassiveObserver);
        assert!(!digest.can_halt());
        assert!(!digest.can_arbitrate());
    }

    #[test]
    fn manifest_declares_read_only_local_rust_inproc_shape() {
        let manifest: toml::Value = toml::from_str(include_str!("../manifest.toml")).unwrap();
        assert_eq!(manifest["class"]["forms"][0].as_str(), Some("rust-inproc"));
        assert_eq!(manifest["class"]["trust_tier"].as_str(), Some("local"));
        assert_eq!(manifest["posture"]["default"].as_str(), Some("cautious"));
        assert_eq!(
            manifest["posture"]["allowed_max"].as_str(),
            Some("cautious")
        );
        assert_eq!(
            manifest["output_shape"]["required_fields"]
                .as_array()
                .map(Vec::len),
            Some(8)
        );
    }

    fn raw_fixture() -> RawDigestInputs {
        let frame_counts = [6, 7, 5, 8, 4, 9, 3, 5];
        let summaries = frame_counts
            .into_iter()
            .enumerate()
            .map(|(index, frames)| SummaryEvidence {
                member: format!("host-{index}"),
                request_id: format!("request-{index}"),
                summary: DigestSummary {
                    frames,
                    halts: u64::from(index < 3),
                    conflicts: u64::from(index == 0),
                },
                source_log_ref: format!("{:032x}", index + 1),
            })
            .collect();
        RawDigestInputs {
            summaries,
            receipt_presence: (0..3)
                .map(|index| HaltReceiptEvidence {
                    member: format!("host-{index}"),
                    halt_id: format!("halt-{index}"),
                    architectural_conflict: index == 0,
                    source_log_ref: format!("{:032x}", index + 20),
                })
                .collect(),
            consent_journal: vec![
                ConsentJournalEvidence::resolved("consult-1", format!("{:032x}", 30)),
                ConsentJournalEvidence::resolved("consult-2", format!("{:032x}", 31)),
                ConsentJournalEvidence::refused("consult-3", format!("{:032x}", 32)),
            ],
        }
    }

    #[test]
    fn narrative_reconciles_three_sources_and_halt_conflict() {
        let derive: fn(&RawDigestInputs) -> Result<TeamDigest, DigestError> = derive_team_digest;
        let digest = derive(&raw_fixture()).unwrap();

        assert_eq!(digest.agents_ran, 8);
        assert_eq!(digest.frames_exchanged, 47);
        assert_eq!(digest.agents_halted, 3);
        assert_eq!(digest.acted_invisibly, 0);
        assert_eq!(digest.consultations_resolved, 2);
        assert_eq!(digest.consultations_refused, 1);
        assert_eq!(digest.architectural_conflicts, 1);
        assert!(digest.narrative.contains("Overnight, 8 agents ran."));
        assert!(digest.narrative.contains("47 IAC frames exchanged."));
        assert!(digest
            .narrative
            .contains("3 agents halted, 0 acted invisibly."));
        assert!(digest
            .narrative
            .contains("2 cross-agent consultations resolved without escalation."));
        assert!(digest
            .narrative
            .contains("1 architectural conflict surfaced for review."));
        assert_eq!(digest.clause_sources.len(), 5);
    }

    #[test]
    fn distillation_request_carries_owned_clause_sources_at_depth_one() {
        let derive: fn(&RawDigestInputs) -> Result<TeamDigest, DigestError> = derive_team_digest;
        let digest = derive(&raw_fixture()).unwrap();
        let request = digest_to_distillation_request(&digest).unwrap();

        assert_eq!(request.distillation_depth, 1);
        assert_eq!(request.source_log_ref.len(), 14);
        assert!(matches!(
            request.digest_payload,
            maos_domain::distillation::DigestPayload::Json(_)
        ));
    }

    #[test]
    fn receipt_for_one_member_cannot_hide_another_members_unreceipted_halt() {
        let mut raw = raw_fixture();
        for summary in &mut raw.summaries {
            summary.summary.halts = u64::from(summary.member == "host-0");
        }
        raw.receipt_presence = vec![HaltReceiptEvidence {
            member: "host-1".into(),
            halt_id: "unrelated-halt".into(),
            architectural_conflict: false,
            source_log_ref: format!("{:032x}", 99),
        }];

        let derive: fn(&RawDigestInputs) -> Result<TeamDigest, DigestError> = derive_team_digest;
        let digest = derive(&raw).unwrap();
        assert_eq!(digest.agents_halted, 1);
        assert_eq!(
            digest.acted_invisibly, 1,
            "receipt presence must reconcile per member, never cancel another member's halt"
        );
    }

    #[test]
    fn member_read_under_two_request_ids_is_counted_once() {
        // A replay under a fresh correlation id must not double-count a
        // member's frames nor double-subtract its receipt presence.
        let mut raw = raw_fixture();
        for summary in &mut raw.summaries {
            summary.summary.halts = u64::from(summary.member == "host-0");
        }
        raw.receipt_presence.clear();
        raw.summaries.push(SummaryEvidence {
            request_id: "request-0-retry".into(),
            ..raw.summaries[0].clone()
        });

        let derive: fn(&RawDigestInputs) -> Result<TeamDigest, DigestError> = derive_team_digest;
        let digest = derive(&raw).unwrap();
        assert_eq!(
            digest.agents_ran, 8,
            "a member read twice is still one agent"
        );
        assert_eq!(
            digest.frames_exchanged, 47,
            "frames must not double-count the replayed summary"
        );
        assert_eq!(
            digest.acted_invisibly, 1,
            "the single unreceipted halt is invisible once, not once per request_id"
        );
    }

    #[test]
    fn member_read_under_two_request_ids_with_conflicting_stats_is_rejected() {
        let mut raw = raw_fixture();
        let mut conflicting = raw.summaries[0].clone();
        conflicting.summary.frames += 1;
        conflicting.request_id = "request-0-conflict".into();
        raw.summaries.push(conflicting);

        let derive: fn(&RawDigestInputs) -> Result<TeamDigest, DigestError> = derive_team_digest;
        assert!(matches!(
            derive(&raw),
            Err(DigestError::ConflictingEvidence(_))
        ));
    }
    fn frame_id_hex(frame_id: [u8; 16]) -> String {
        frame_id.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn real_writer_accepts_owned_evidence_and_rejects_peer_private_evidence() {
        use maos_domain::distillation::DistillationError;
        use maos_domain::invariants::i3::FrameOrigin;
        use maos_iac::adapter::distillate::DistillateWriter;
        use maos_iac::adapter::transparency_log::{FrameKind, TransparencyLogAdapter};

        let log = Arc::new(TransparencyLogAdapter::open_in_memory(0xD16357));
        let memory: Arc<dyn std::any::Any + Send + Sync> = Arc::new(());
        let writer = DistillateWriter::new(log.clone(), memory);
        let mut raw = raw_fixture();
        for source in raw
            .summaries
            .iter_mut()
            .map(|evidence| &mut evidence.source_log_ref)
            .chain(
                raw.receipt_presence
                    .iter_mut()
                    .map(|evidence| &mut evidence.source_log_ref),
            )
            .chain(
                raw.consent_journal
                    .iter_mut()
                    .map(|evidence| &mut evidence.source_log_ref),
            )
        {
            log.insert_frame_event(
                FrameKind::TelemetryEvent,
                77,
                None,
                "cohort:digest-ingestion",
                b"consented evidence",
                FrameOrigin::SpiritAuto,
            );
            *source = frame_id_hex(log.last_frame_id());
        }

        let derive: fn(&RawDigestInputs) -> Result<TeamDigest, DigestError> = derive_team_digest;
        let digest = derive(&raw).unwrap();
        let receipt = persist_team_digest(&writer, 77, &digest).unwrap();
        assert_eq!(receipt.effective_source_log_ref.len(), 14);

        log.insert_frame_event(
            FrameKind::TelemetryEvent,
            88,
            None,
            "private-peer-frame",
            b"must not be cited",
            FrameOrigin::SpiritAuto,
        );
        raw.summaries[0].source_log_ref = frame_id_hex(log.last_frame_id());
        let forged = derive(&raw).unwrap();
        assert!(matches!(
            persist_team_digest(&writer, 77, &forged),
            Err(DigestError::Distillation(
                DistillationError::CiterUnauthorized {
                    citer_pid: 77,
                    source_pid: 88,
                    ..
                }
            ))
        ));
    }
}
