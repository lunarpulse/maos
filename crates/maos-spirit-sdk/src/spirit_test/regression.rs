#![forbid(unsafe_code)]

//! Class-specific regression corpus skeleton — typed container.
//!
//! Actual class corpora ship in Stories 8.1–8.5 (Butler / Researcher /
//! Founder-Loop / Mira-Nash reference Spirits). This crate ships the
//! TYPE SHAPE so reference-Spirit authors plug in without inventing
//! the schema.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiritClass {
    /// Architecture §6 Butler — anticipatory single-Spirit (on_idle).
    Anticipatory,
    /// Architecture §6 Researcher — exploratory single-Spirit + distillation.
    Exploratory,
    /// Architecture §6 Orchestrator + Worker + Architect + Reviewer wedge.
    FounderLoop,
    /// Architecture §6 Mira + Nash — diagnostic-architect bilateral pair.
    DiagnosticArchitect,
    /// Third-party Spirit class — Diego J6 onboarding persona.
    Generic,
}

#[derive(Debug, Clone)]
pub struct RegressionCase {
    pub id: String,
    pub fixture_setup: String,
    pub expected_assertions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegressionCorpus {
    pub class: SpiritClass,
    pub cases: Vec<RegressionCase>,
}

impl RegressionCorpus {
    pub fn new(class: SpiritClass) -> Self {
        Self { class, cases: Vec::new() }
    }
}
