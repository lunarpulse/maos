#![forbid(unsafe_code)]

//! `architect` — MAOS Architect v0.8, a founder-loop reference Spirit. Story 8.4.
//!
//! The Architect **proposes a design** from a task spec in the founder-loop
//! code-review loop ([`Architect::propose`]). Its cognition is **deterministic /
//! seeded / bit-identical** at v0.8 (NFR-Testability-1) — same spec ⇒ identical
//! proposal, no live LLM in CI. Live generative behaviour is application-layer
//! (v0.9+/Epic 9 — Decision E).
//!
//! The Architect is a **specialized Worker** — it carries `SpiritRole::Worker`
//! in its `FrameAddress.role` (Decision C); there is NO `Architect` ABI variant
//! (the v1.0 ABI is frozen). In the loop, the Architect emits its proposal as a
//! `task.complete`, then **distills its OWN output** (Decision K) so the
//! Orchestrator can reference the distillate when dispatching to the Reviewer.
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! Spirit-side deps only (`maos-spirit-sdk` + `maos-spirit-abi` + `maos-domain`
//! + serde). The real FR21 gate + `DistillateWriter` are reached as
//! dev-dependencies in `tests/` only.

use std::sync::{Arc, Mutex};

use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// A design proposal — serialized to exactly the manifest `[output_shape]`
/// `required_fields` (`components` / `rationale` / `interfaces` / `risks`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignProposal {
    /// One component per requirement extracted from the spec.
    pub components: Vec<String>,
    /// A deterministic rationale summarizing the design.
    pub rationale: String,
    /// The interface surface the design exposes.
    pub interfaces: Vec<String>,
    /// Risks the Reviewer should scrutinize (drives the critique).
    pub risks: Vec<String>,
}

impl DesignProposal {
    /// A compact, deterministic text rendering used as the distillate digest.
    pub fn digest_text(&self) -> String {
        format!(
            "design: {} components [{}]; interfaces [{}]; risks [{}]; {}",
            self.components.len(),
            self.components.join(", "),
            self.interfaces.join(", "),
            self.risks.join(", "),
            self.rationale
        )
    }
}

/// Architect reference Spirit — proposes a design deterministically. `Arc<Mutex>`
/// interior state keeps it `Sync` for the `#[spirit]` macro.
#[derive(Debug, Clone)]
pub struct Architect {
    architect_id: String,
    /// A seeded spec an `on_idle` pass proposes against (fixture / harness path).
    pending_spec: Option<String>,
    /// The proposal produced by the most recent `on_idle` pass.
    last_proposal: Arc<Mutex<Option<DesignProposal>>>,
}

#[spirit]
impl Architect {
    /// Idle design pass. Cancellation-aware and bounded (a single deterministic
    /// transform over the seeded spec). Stores the proposal so the hook has a
    /// production-visible effect.
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        if let Some(spec) = &self.pending_spec {
            let proposal = self.propose(spec);
            let mut guard = self.last_proposal.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(proposal);
        }
    }
}

impl Default for Architect {
    fn default() -> Self {
        Self {
            architect_id: "architect".to_string(),
            pending_spec: None,
            last_proposal: Arc::new(Mutex::new(None)),
        }
    }
}

impl Architect {
    /// A fresh Architect with the given Spirit id.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            architect_id: id.into(),
            ..Self::default()
        }
    }

    /// Seed a spec an `on_idle` pass will propose against (fixture / harness).
    pub fn with_pending_spec(mut self, spec: impl Into<String>) -> Self {
        self.pending_spec = Some(spec.into());
        self
    }

    /// This Architect's own Spirit id.
    pub fn architect_id(&self) -> &str {
        &self.architect_id
    }

    /// The proposal produced by the most recent `on_idle` pass.
    pub fn last_proposal(&self) -> Option<DesignProposal> {
        self.last_proposal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Propose a design from `spec` — **deterministic / seeded / bit-identical**
    /// (same spec ⇒ same proposal). Requirements are the `;`-separated (or
    /// whitespace-fallback) clauses of the spec; each becomes one component with
    /// a derived interface. A clause containing "unsafe", "global", or "mutable"
    /// is flagged as a risk (the deterministic stand-in for design review at
    /// v0.8; live reasoning is v0.9+).
    pub fn propose(&self, spec: &str) -> DesignProposal {
        let clauses: Vec<&str> = if spec.contains(';') {
            spec.split(';')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .collect()
        } else {
            spec.split_whitespace().collect()
        };

        let mut components = Vec::new();
        let mut interfaces = Vec::new();
        let mut risks = Vec::new();
        for clause in &clauses {
            let slug: String = clause
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_string();
            let slug = if slug.is_empty() {
                "component".to_string()
            } else {
                slug
            };
            components.push(format!("{slug}-module"));
            interfaces.push(format!("{slug}_port"));
            let lc = clause.to_ascii_lowercase();
            if lc.contains("unsafe") || lc.contains("global") || lc.contains("mutable") {
                risks.push(format!("{slug}: shared-state hazard"));
            }
        }
        if components.is_empty() {
            components.push("core-module".to_string());
            interfaces.push("core_port".to_string());
        }

        DesignProposal {
            rationale: format!(
                "decomposed the spec into {} component(s) with explicit port interfaces",
                components.len()
            ),
            components,
            interfaces,
            risks,
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn propose_is_deterministic_bit_identical() {
        let a = Architect::new("architect");
        let spec = "parse input; validate schema; persist record";
        let p1 = a.propose(spec);
        let p2 = a.propose(spec);
        assert_eq!(p1, p2, "same spec ⇒ bit-identical proposal");
        assert_eq!(p1.components.len(), 3);
        assert!(p1.interfaces.iter().any(|i| i == "parse-input_port"));
    }

    #[test]
    fn propose_flags_shared_state_risks() {
        let a = Architect::new("architect");
        let p = a.propose("mutate global counter; read config");
        assert!(
            !p.risks.is_empty(),
            "a clause touching global/mutable state is flagged"
        );
    }

    #[test]
    fn empty_spec_still_proposes_a_core_module() {
        let a = Architect::new("architect");
        let p = a.propose("");
        assert_eq!(p.components, vec!["core-module".to_string()]);
    }

    #[test]
    fn proposal_serializes_with_required_output_fields() {
        let a = Architect::new("architect");
        let p = a.propose("design the founder loop");
        let v = serde_json::to_value(&p).unwrap();
        for field in ["components", "rationale", "interfaces", "risks"] {
            assert!(v.get(field).is_some(), "missing output field {field}");
        }
    }

    #[test]
    fn on_idle_proposes_against_seeded_spec() {
        use maos_spirit_sdk::spirit_test::SpiritTest;
        let spirit = Architect::new("architect").with_pending_spec("parse; validate");
        let vtable = __maos_spirit_vtable_Architect();
        let mut harness = SpiritTest::new(&spirit, &vtable);
        harness.fixture_mut().invoke_on_idle = true;
        let _ = harness.run();
        let p = spirit.last_proposal().expect("on_idle produced a proposal");
        assert_eq!(p.components.len(), 2);
    }
}
