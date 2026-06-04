#![forbid(unsafe_code)]

//! `reviewer` — MAOS Reviewer v0.8, a founder-loop reference Spirit. Story 8.4.
//!
//! The Reviewer **critiques the Architect's design proposal**
//! ([`Reviewer::review`]) in the founder-loop code-review loop. Its cognition is
//! **deterministic / seeded / bit-identical** at v0.8 (NFR-Testability-1) — same
//! design ⇒ identical critique, no live LLM in CI. Live generative behaviour is
//! application-layer (v0.9+/Epic 9 — Decision E).
//!
//! The Reviewer is a **specialized Worker** — it carries `SpiritRole::Worker`
//! (Decision C); there is NO `Reviewer` ABI variant (the v1.0 ABI is frozen). It
//! reviews a structural [`DesignUnderReview`] (its OWN input type — the crate is
//! decoupled from `architect`), then distills its OWN critique (Decision K) so
//! the critique flows back through the Orchestrator.

use std::sync::{Arc, Mutex};

use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// The structural design the Reviewer critiques. Decoupled from `architect`'s
/// `DesignProposal` so the crates do not depend on each other; the loop maps one
/// to the other at the seam.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DesignUnderReview {
    /// The proposed components.
    pub components: Vec<String>,
    /// The proposed interfaces.
    pub interfaces: Vec<String>,
    /// The risks the Architect flagged (the primary critique driver).
    pub risks: Vec<String>,
}

/// A critique — serialized to exactly the manifest `[output_shape]`
/// `required_fields` (`findings` / `verdict` / `severity` / `summary`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Critique {
    /// One finding per risk (or an "approve" note when the design is clean).
    pub findings: Vec<String>,
    /// `"approve"` or `"changes-requested"`.
    pub verdict: String,
    /// `"none"` / `"low"` / `"high"`.
    pub severity: String,
    /// A deterministic human-readable summary.
    pub summary: String,
}

impl Critique {
    /// Whether the Reviewer approved the design.
    pub fn approved(&self) -> bool {
        self.verdict == "approve"
    }

    /// A compact, deterministic text rendering used as the distillate digest.
    pub fn digest_text(&self) -> String {
        format!(
            "review: {} [{}]; {} finding(s) [{}]; {}",
            self.verdict,
            self.severity,
            self.findings.len(),
            self.findings.join(", "),
            self.summary
        )
    }
}

/// Reviewer reference Spirit — critiques a design deterministically. `Arc<Mutex>`
/// interior state keeps it `Sync` for the `#[spirit]` macro.
#[derive(Debug, Clone)]
pub struct Reviewer {
    reviewer_id: String,
    /// A seeded design an `on_idle` pass critiques (fixture / harness path).
    pending_design: Option<DesignUnderReview>,
    /// The critique produced by the most recent `on_idle` pass.
    last_critique: Arc<Mutex<Option<Critique>>>,
}

#[spirit]
impl Reviewer {
    /// Idle review pass. Cancellation-aware and bounded (a single deterministic
    /// transform over the seeded design). Stores the critique so the hook has a
    /// production-visible effect.
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        if let Some(design) = &self.pending_design {
            let critique = self.review(design);
            let mut guard = self.last_critique.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(critique);
        }
    }
}

impl Default for Reviewer {
    fn default() -> Self {
        Self {
            reviewer_id: "reviewer".to_string(),
            pending_design: None,
            last_critique: Arc::new(Mutex::new(None)),
        }
    }
}

impl Reviewer {
    /// A fresh Reviewer with the given Spirit id.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            reviewer_id: id.into(),
            ..Self::default()
        }
    }

    /// Seed a design an `on_idle` pass will critique (fixture / harness).
    pub fn with_pending_design(mut self, design: DesignUnderReview) -> Self {
        self.pending_design = Some(design);
        self
    }

    /// This Reviewer's own Spirit id.
    pub fn reviewer_id(&self) -> &str {
        &self.reviewer_id
    }

    /// The critique produced by the most recent `on_idle` pass.
    pub fn last_critique(&self) -> Option<Critique> {
        self.last_critique
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Critique a design — **deterministic / seeded / bit-identical** (same
    /// design ⇒ same critique). Each flagged risk becomes a `changes-requested`
    /// finding (severity `high`); a risk-free design with a port interface per
    /// component is `approve`d (severity `none`); a design missing interface
    /// coverage draws a `low`-severity coverage finding.
    pub fn review(&self, design: &DesignUnderReview) -> Critique {
        let mut findings = Vec::new();
        for risk in &design.risks {
            findings.push(format!("address risk: {risk}"));
        }
        // Interface-coverage check: each component should expose a port.
        if design.interfaces.len() < design.components.len() {
            findings.push(format!(
                "interface coverage: {} component(s) but only {} interface(s)",
                design.components.len(),
                design.interfaces.len()
            ));
        }

        if findings.is_empty() {
            Critique {
                findings: vec![
                    "design is sound — interfaces cover all components, no flagged risks"
                        .to_string(),
                ],
                verdict: "approve".to_string(),
                severity: "none".to_string(),
                summary: format!(
                    "approve: {} component(s), {} interface(s), 0 risks",
                    design.components.len(),
                    design.interfaces.len()
                ),
            }
        } else {
            let severity = if design.risks.is_empty() {
                "low"
            } else {
                "high"
            };
            Critique {
                summary: format!(
                    "changes-requested: {} finding(s) across {} risk(s)",
                    findings.len(),
                    design.risks.len()
                ),
                findings,
                verdict: "changes-requested".to_string(),
                severity: severity.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn clean_design() -> DesignUnderReview {
        DesignUnderReview {
            components: vec!["a-module".into(), "b-module".into()],
            interfaces: vec!["a_port".into(), "b_port".into()],
            risks: vec![],
        }
    }

    #[test]
    fn review_is_deterministic_bit_identical() {
        let r = Reviewer::new("reviewer");
        let d = clean_design();
        assert_eq!(
            r.review(&d),
            r.review(&d),
            "same design ⇒ bit-identical critique"
        );
    }

    #[test]
    fn clean_design_is_approved() {
        let r = Reviewer::new("reviewer");
        let c = r.review(&clean_design());
        assert!(c.approved());
        assert_eq!(c.severity, "none");
    }

    #[test]
    fn risky_design_draws_high_severity_changes() {
        let r = Reviewer::new("reviewer");
        let d = DesignUnderReview {
            components: vec!["x-module".into()],
            interfaces: vec!["x_port".into()],
            risks: vec!["x: shared-state hazard".into()],
        };
        let c = r.review(&d);
        assert!(!c.approved());
        assert_eq!(c.verdict, "changes-requested");
        assert_eq!(c.severity, "high");
        assert!(c.findings.iter().any(|f| f.contains("shared-state hazard")));
    }

    #[test]
    fn missing_interface_coverage_draws_low_severity() {
        let r = Reviewer::new("reviewer");
        let d = DesignUnderReview {
            components: vec!["a-module".into(), "b-module".into()],
            interfaces: vec!["a_port".into()],
            risks: vec![],
        };
        let c = r.review(&d);
        assert_eq!(c.verdict, "changes-requested");
        assert_eq!(c.severity, "low");
    }

    #[test]
    fn critique_serializes_with_required_output_fields() {
        let r = Reviewer::new("reviewer");
        let c = r.review(&clean_design());
        let v = serde_json::to_value(&c).unwrap();
        for field in ["findings", "verdict", "severity", "summary"] {
            assert!(v.get(field).is_some(), "missing output field {field}");
        }
    }

    #[test]
    fn on_idle_reviews_seeded_design() {
        use maos_spirit_sdk::spirit_test::SpiritTest;
        let spirit = Reviewer::new("reviewer").with_pending_design(clean_design());
        let vtable = __maos_spirit_vtable_Reviewer();
        let mut harness = SpiritTest::new(&spirit, &vtable);
        harness.fixture_mut().invoke_on_idle = true;
        let _ = harness.run();
        assert!(spirit
            .last_critique()
            .expect("on_idle produced a critique")
            .approved());
    }
}
