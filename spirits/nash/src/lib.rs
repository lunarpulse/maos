#![forbid(unsafe_code)]

//! `nash` — MAOS Nash v1.5, the dev-environment **Senior Architect** reference
//! Spirit and **Host B** of the Story 8.5 bilateral diagnostic-architect pair
//! (architecture §6.4).
//!
//! Nash is a **specialized Worker** (`SpiritRole::Worker` — Decision C; the role
//! is set in the [`FrameAddress`](maos_domain::frame::FrameAddress)`.role` at
//! registration, not a manifest field). It:
//!
//! 1. **Receives Mira's cross-Host diagnostic advisory** via A2A typed-intent
//!    consent (ADR-012): the advisory frame Mira routes over the real
//!    `LoopbackA2ARouter` carries Mira's [`DiagnosticAdvisory`] as a JSON wire
//!    payload. Nash deserializes it into [`AdvisoryInput`] ([`Nash::from_wire`]) —
//!    the cross-Host contract is the serde shape (no crate coupling to `mira`),
//!    exactly as a real two-process frame would be.
//! 2. **Proposes an architecture fix deterministically** ([`Nash::architect`]) —
//!    no live LLM at v1.5 (Decision I); the proposal is pure, seeded, and
//!    bit-identical (NFR-Testability-1).
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! This crate depends only on the Spirit SDK/ABI and serde. The cross-Host A2A
//! integration (Mira → Nash advisory over the real router + TOFU + consent) is
//! proven in `spirits/mira/tests/` (Mira is the J4 journey's protagonist), which
//! carries the real adapters as dev-dependencies only.

use std::sync::{Arc, Mutex};

use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// The cross-Host advisory Nash receives from Mira (the A2A wire payload). The
/// field names mirror `mira::DiagnosticAdvisory` exactly so a frame Mira
/// serialized deserializes here without a crate dependency (the genuine
/// cross-Host contract is the serde shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvisoryInput {
    /// The prod-edge subject Mira's advisory concerns.
    pub subject: String,
    /// Mira's finding (read-only diagnostic evidence).
    pub finding: String,
    /// Severity in `[0.0, 1.0]`.
    pub severity: f64,
    /// What Mira recommends Nash investigate / architect.
    pub recommended_action: String,
    /// The Transparency-Log reference the advisory cites (FR17).
    pub source_log_ref: String,
}

/// Nash's deterministic architecture proposal. Serializes to exactly the manifest
/// `[output_shape]` `required_fields`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchitectureProposal {
    /// The prod-edge subject the proposal addresses.
    pub subject: String,
    /// A short restatement of the diagnosis Nash is architecting against.
    pub diagnosis_summary: String,
    /// The proposed fix.
    pub proposed_fix: String,
    /// The components the fix touches.
    pub components: Vec<String>,
    /// Nash's confidence in the proposal in `[0.0, 1.0]`.
    pub confidence: f64,
    /// The source-log reference threaded from Mira's advisory (FR17 citation).
    #[serde(default)]
    pub source_log_ref: String,
}

// ───────────────────────────────────────────────────────────────────────────
// The Nash Spirit.
// ───────────────────────────────────────────────────────────────────────────

/// Nash reference Spirit — a dev-environment Senior Architect. Holds its own
/// Spirit id and an optional seeded batch of pending advisories an `on_idle` pass
/// architects. `Arc<Mutex<...>>` interior state keeps Nash `Sync` as the
/// `#[spirit]` macro requires (poison-safe `unwrap_or_else(|e| e.into_inner())`).
#[derive(Debug, Clone)]
pub struct Nash {
    /// This Nash's own Spirit id.
    spirit_id: String,
    /// Optional seeded advisories an `on_idle` pass architects (fixture / harness).
    pending_advisories: Option<Vec<AdvisoryInput>>,
    /// Proposals produced by the most recent `on_idle` pass.
    last_proposals: Arc<Mutex<Vec<ArchitectureProposal>>>,
}

#[spirit]
impl Nash {
    /// Idle architecture pass. Cancellation-aware; bounded (a single linear pass
    /// over any seeded advisories). Stores the resulting proposals so the hook has
    /// a production-visible effect; the LIVE A2A receive path is proven against
    /// the real `LoopbackA2ARouter` in `spirits/mira/tests/`.
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        let mut proposals = Vec::new();
        if let Some(advisories) = &self.pending_advisories {
            for adv in advisories {
                proposals.push(self.architect(adv));
            }
        }
        let mut guard = self.last_proposals.lock().unwrap_or_else(|e| e.into_inner());
        *guard = proposals;
    }
}

impl Default for Nash {
    fn default() -> Self {
        Self {
            spirit_id: "nash".to_string(),
            pending_advisories: None,
            last_proposals: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Nash {
    /// Override this Nash's own Spirit id.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.spirit_id = id.into();
        self
    }

    /// Seed advisories an `on_idle` pass will architect (fixture / harness).
    pub fn with_pending_advisories(mut self, advisories: Vec<AdvisoryInput>) -> Self {
        self.pending_advisories = Some(advisories);
        self
    }

    /// This Nash's own Spirit id.
    pub fn spirit_id(&self) -> &str {
        &self.spirit_id
    }

    /// Proposals produced by the most recent `on_idle` pass.
    pub fn last_proposals(&self) -> Vec<ArchitectureProposal> {
        self.last_proposals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Deserialize an advisory off the A2A wire (the JSON Mira's frame carried).
    pub fn from_wire(payload: &str) -> Result<AdvisoryInput, serde_json::Error> {
        serde_json::from_str(payload)
    }

    // ── deterministic architecture (AC4) ──────────────────────────────────────

    /// Architect a fix for one diagnostic advisory. **Deterministic** — pure
    /// function of the advisory (Decision I; NFR-Testability-1). The proposed fix
    /// + touched components are selected by the advisory's severity; confidence
    /// falls as severity rises (a more severe anomaly is a harder architecture
    /// problem). Threads Mira's `source_log_ref` so the morning digest can cite
    /// it (FR17).
    pub fn architect(&self, advisory: &AdvisoryInput) -> ArchitectureProposal {
        let severity = if advisory.severity.is_nan() {
            0.0
        } else {
            advisory.severity.clamp(0.0, 1.0)
        };
        let (proposed_fix, components) = if severity >= 0.66 {
            (
                format!(
                    "isolate '{}' behind a circuit-breaker, add a fallback path, and instrument the failure mode with new observability",
                    advisory.subject
                ),
                vec![
                    advisory.subject.clone(),
                    "circuit-breaker".to_string(),
                    "fallback".to_string(),
                    "observability".to_string(),
                ],
            )
        } else if severity >= 0.33 {
            (
                format!(
                    "add a bounded retry + backoff around '{}' and raise an alert on recurrence",
                    advisory.subject
                ),
                vec![
                    advisory.subject.clone(),
                    "retry-backoff".to_string(),
                    "alerting".to_string(),
                ],
            )
        } else {
            (
                format!(
                    "tune the thresholds for '{}' and add a regression test capturing the baseline",
                    advisory.subject
                ),
                vec![advisory.subject.clone(), "regression-test".to_string()],
            )
        };
        // A more severe (harder) anomaly yields a less-confident architecture.
        let confidence = (0.95 - severity * 0.35).clamp(0.0, 1.0);
        ArchitectureProposal {
            subject: advisory.subject.clone(),
            diagnosis_summary: advisory.finding.clone(),
            proposed_fix,
            components,
            confidence,
            source_log_ref: advisory.source_log_ref.clone(),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn advisory(severity: f64) -> AdvisoryInput {
        AdvisoryInput {
            subject: "edge-cache".into(),
            finding: "prod-edge anomaly on 'edge-cache': novel_entropy_drift".into(),
            severity,
            recommended_action: "architect a mitigation".into(),
            source_log_ref: "tl:row:2002".into(),
        }
    }

    #[test]
    fn high_severity_proposes_circuit_breaker() {
        let n = Nash::default();
        let p = n.architect(&advisory(0.9));
        assert_eq!(p.subject, "edge-cache");
        assert!(p.proposed_fix.contains("circuit-breaker"));
        assert!(p.components.contains(&"circuit-breaker".to_string()));
        assert!(p.components.contains(&"observability".to_string()));
        assert_eq!(p.source_log_ref, "tl:row:2002");
    }

    #[test]
    fn mid_severity_proposes_retry_backoff() {
        let n = Nash::default();
        let p = n.architect(&advisory(0.4));
        assert!(p.proposed_fix.contains("retry"));
        assert!(p.components.contains(&"retry-backoff".to_string()));
    }

    #[test]
    fn low_severity_proposes_threshold_tuning() {
        let n = Nash::default();
        let p = n.architect(&advisory(0.1));
        assert!(p.proposed_fix.contains("threshold") || p.proposed_fix.contains("regression"));
        assert!(p.components.contains(&"regression-test".to_string()));
    }

    #[test]
    fn confidence_falls_with_severity() {
        let n = Nash::default();
        let low = n.architect(&advisory(0.1)).confidence;
        let high = n.architect(&advisory(0.9)).confidence;
        assert!(high < low, "a harder (more severe) anomaly is less confident");
    }

    #[test]
    fn architecture_is_deterministic() {
        let n = Nash::default();
        let a = n.architect(&advisory(0.7));
        let b = n.architect(&advisory(0.7));
        assert_eq!(a, b, "architecture is bit-identical (NFR-Testability-1)");
    }

    #[test]
    fn proposal_serializes_with_required_output_fields() {
        let n = Nash::default();
        let p = n.architect(&advisory(0.5));
        let v = serde_json::to_value(&p).unwrap();
        for field in ["subject", "proposed_fix", "components", "confidence"] {
            assert!(v.get(field).is_some(), "missing required output field {field}");
        }
    }

    #[test]
    fn from_wire_round_trips_mira_advisory_shape() {
        // The exact JSON a `mira::DiagnosticAdvisory` serializes to.
        let wire = r#"{
            "subject": "edge-cache",
            "finding": "prod-edge anomaly",
            "severity": 0.8,
            "recommended_action": "architect a mitigation",
            "source_log_ref": "tl:row:2002"
        }"#;
        let adv = Nash::from_wire(wire).expect("valid advisory wire");
        assert_eq!(adv.subject, "edge-cache");
        let p = Nash::default().architect(&adv);
        assert!(p.proposed_fix.contains("circuit-breaker"));
    }

    #[test]
    fn from_wire_rejects_malformed_json() {
        assert!(Nash::from_wire("{not json").is_err());
    }

    #[test]
    fn nan_severity_does_not_panic() {
        let n = Nash::default();
        let p = n.architect(&advisory(f64::NAN));
        assert!(p.confidence >= 0.0 && p.confidence <= 1.0);
    }

    #[test]
    fn with_id_overrides_spirit_id() {
        let n = Nash::default().with_id("nash-host-b");
        assert_eq!(n.spirit_id(), "nash-host-b");
    }
}
