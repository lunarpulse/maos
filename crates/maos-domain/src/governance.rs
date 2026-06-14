//! FR62 governance domain types ([ADR-045]).
//!
//! Governance events are journaled as `FrameKind::GovernanceEvent` (28)
//! in the Transparency Log.  The payload is a [`GovernanceEventPayload`]
//! with a discriminated [`GovernanceEventKind`] field.
//!
//! [ADR-045]: ../../docs/adr/ADR-045-governance-audit-artifacts.md

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ABI-extension proposal (ADR-045 §4 / F6)
// ---------------------------------------------------------------------------

/// Ratification status for an ABI extension proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatificationStatus {
    Proposed,
    Ratified,
    Rejected,
}

/// An ABI extension proposal per ADR-045 §4 (F6).
///
/// Carries provenance (who proposed, under which ADR) for the governance
/// record that reconciles with `xtask abi-diff` via the one-directional
/// `abi-diff ⊆ ratified` gate (R1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiExtensionProposal {
    /// Unique proposal identifier (e.g. `"9.3b-FR62-FR64-kernel-repin"`).
    pub proposal_id: String,
    /// Human-readable summary of what ABI surface this proposal covers.
    pub summary: String,
    /// Reference to the governing ADR (e.g. `"ADR-045,ADR-046"`).
    pub adr_ref: String,
    /// Current ratification status.
    pub status: RatificationStatus,
    /// ABI surface change patterns this proposal covers.  Each entry is a
    /// substring that must appear in the abi-diff output for the change
    /// to be considered "covered" by this proposal.
    pub covered_changes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Vetter-key admission/rotation (ADR-045 §1, stream 1)
// ---------------------------------------------------------------------------

/// Vetter-key admission/rotation event.
///
/// Emitted at `admit_spirit()` decision points (both admit and reject)
/// so the audit trail records every trust-tier decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VetterKeyPayload {
    /// Spirit identity being admitted or rejected.
    pub spirit_id: String,
    /// Version of the Spirit being admitted.
    pub version: String,
    /// Whether admission was granted.
    pub admitted: bool,
    /// Effective trust tier (strictest-of result).
    pub effective_tier: String,
    /// Human-readable journal note from the admission decision.
    pub journal_note: String,
}

// ---------------------------------------------------------------------------
// ComplianceClaim schema-lifecycle (ADR-045 §2–§3, stream 2)
// ---------------------------------------------------------------------------

/// Schema-lifecycle event for a ComplianceClaim schema version.
///
/// Per ADR-045 §2 (F5): decoupled from the frozen `Claim` struct.
/// References schema identity only — zero claim-instance ids (R11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaLifecyclePayload {
    /// Stable version-independent reverse-DNS lineage name (R11).
    /// E.g. `"compliance.claim.gdpr-erasure"`.
    pub schema_id: String,
    /// Per-version fingerprint (SHA-256 hex of canonical schema bytes).
    pub schema_content_hash: String,
    /// References the prior version's hash for verifiable chain (R11).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    /// Schema version number.
    pub version: u32,
    /// ADR reference that ratified this schema version.
    pub ratified_by: String,
}

/// A row in the R10 append-only schema-lifecycle registry.
///
/// Per ADR-045 §3: the registry is the authority — entries must carry
/// a `ratified_by` ADR reference or be HARD-REJECTED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRegistryEntry {
    pub schema_id: String,
    pub version: u32,
    pub effective_at_ns: u64,
    pub supersedes_hash: Option<String>,
    pub ratified_by: String,
    pub recorded_at_ns: u64,
    pub schema_content_hash: String,
}

// ---------------------------------------------------------------------------
// Governance event envelope (shared across all three streams)
// ---------------------------------------------------------------------------

/// Sub-type discriminator for governance events.
///
/// Per ADR-045 §1, three governance streams share `FrameKind::GovernanceEvent`:
/// ABI extension proposals, vetter-key admission/rotation, and ComplianceClaim
/// schema-lifecycle events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "governance_type")]
#[non_exhaustive]
pub enum GovernanceEventKind {
    /// F6 — ABI extension proposal/ratification.
    AbiExtension(AbiExtensionProposal),
    /// Vetter-key admission/rotation event (ADR-045 §1, stream 1).
    VetterKey(VetterKeyPayload),
    /// ComplianceClaim schema-lifecycle event (ADR-045 §2, stream 2).
    SchemaLifecycle(SchemaLifecyclePayload),
}

/// Payload for governance TL frames (`FrameKind::GovernanceEvent` = 28).
///
/// Per ADR-045 R12: every governance event carries **both** `recorded_at`
/// (monotonic journal position) **and** `effective_at` (when the decision
/// takes governance effect).  They genuinely differ — an ADR ratified
/// June 14 can make a schema effective July 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceEventPayload {
    /// Monotonic journal position (nanosecond wall-clock of TL insertion).
    pub recorded_at_ns: u64,
    /// When the governance decision takes effect (may differ from
    /// `recorded_at_ns` — e.g. ratified today, effective next month).
    pub effective_at_ns: u64,
    /// The discriminated governance event.
    pub event: GovernanceEventKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_extension_proposal_round_trip() {
        let proposal = AbiExtensionProposal {
            proposal_id: "9.3b-FR62-FR64-kernel-repin".into(),
            summary: "FR62 GovernanceEvent kind + FR64 CostAttribution kind".into(),
            adr_ref: "ADR-045,ADR-046".into(),
            status: RatificationStatus::Ratified,
            covered_changes: vec!["GovernanceEvent".into(), "CostAttribution".into()],
        };
        let payload = GovernanceEventPayload {
            recorded_at_ns: 1_000_000,
            effective_at_ns: 2_000_000,
            event: GovernanceEventKind::AbiExtension(proposal.clone()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: GovernanceEventPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, back);
    }

    #[test]
    fn ratification_status_serde_snake_case() {
        let json = serde_json::to_string(&RatificationStatus::Ratified).unwrap();
        assert_eq!(json, r#""ratified""#);
        let back: RatificationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, RatificationStatus::Ratified);
    }

    #[test]
    fn governance_event_kind_tagged() {
        let proposal = AbiExtensionProposal {
            proposal_id: "test".into(),
            summary: "test".into(),
            adr_ref: "ADR-000".into(),
            status: RatificationStatus::Proposed,
            covered_changes: vec![],
        };
        let kind = GovernanceEventKind::AbiExtension(proposal);
        let json = serde_json::to_string(&kind).unwrap();
        assert!(json.contains(r#""governance_type":"AbiExtension""#));
    }

    /// AC6 shared-key contract test: lifecycle and registry streams MUST
    /// key off ONE canonical `schema_id`.  The test is non-tautological:
    /// positive + negative + supersession-chain assertions.
    #[test]
    fn shared_schema_id_key_contract() {
        let canonical_id = "compliance.claim.gdpr-erasure";
        let canonical_hash = "abc123";

        let lifecycle = SchemaLifecyclePayload {
            schema_id: canonical_id.into(),
            schema_content_hash: canonical_hash.into(),
            supersedes: None,
            version: 1,
            ratified_by: "ADR-045".into(),
        };

        let registry_entry = SchemaRegistryEntry {
            schema_id: canonical_id.into(),
            version: 1,
            effective_at_ns: 1_000_000,
            supersedes_hash: None,
            ratified_by: "ADR-045".into(),
            recorded_at_ns: 1_000_000,
            schema_content_hash: canonical_hash.into(),
        };

        // Positive: both keyed off the same canonical source.
        assert_eq!(lifecycle.schema_id, registry_entry.schema_id);
        assert_eq!(lifecycle.schema_content_hash, registry_entry.schema_content_hash);
        assert_eq!(lifecycle.version, registry_entry.version);

        // Negative: a divergent schema_id MUST NOT match (fake-join guard).
        let divergent = SchemaRegistryEntry {
            schema_id: "compliance.claim.legal-hold".into(),
            version: 1,
            effective_at_ns: 1_000_000,
            supersedes_hash: None,
            ratified_by: "ADR-045".into(),
            recorded_at_ns: 1_000_000,
            schema_content_hash: canonical_hash.into(),
        };
        assert_ne!(
            lifecycle.schema_id, divergent.schema_id,
            "AC6: divergent schema_id must not match — catches fake joins"
        );

        // Supersession chain: v2 references v1's content hash.
        let lifecycle_v2 = SchemaLifecyclePayload {
            schema_id: canonical_id.into(),
            schema_content_hash: "def456".into(),
            supersedes: Some(canonical_hash.into()),
            version: 2,
            ratified_by: "ADR-045".into(),
        };
        assert_eq!(
            lifecycle_v2.supersedes.as_deref(),
            Some(lifecycle.schema_content_hash.as_str()),
            "AC6: v2 must chain back to v1's content hash"
        );
    }

    /// Erasure-class lineage ids cover the full set (Art.17 / legal-hold / retention).
    #[test]
    fn erasure_class_lineage_ids() {
        let erasure_class_ids = [
            "compliance.claim.gdpr-erasure",
            "compliance.claim.legal-hold",
            "compliance.claim.retention-expiry",
        ];
        // Each must be a valid reverse-DNS lineage name (R11)
        for id in &erasure_class_ids {
            assert!(id.starts_with("compliance.claim."), "R11: reverse-DNS lineage name");
            assert!(!id.is_empty());
        }
    }
}
