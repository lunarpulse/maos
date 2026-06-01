#![forbid(unsafe_code)]

//! `maos-eval` — corpora + measurement gates.
//!
//! Hosts the fixture corpora that AC4 (1000-termination), AC6 (50-halt
//! synthetic-v0), and Story 4.5 (200-isolation, 100-HSIS) measure
//! against. The crate is read-only at runtime — corpora ship as
//! files under `fixtures/`; the lib surface provides loaders +
//! schema types only.
//!
//! Dep direction: `maos-eval` depends on `maos-domain` (for
//! `EpistemicHaltPayload` deserialization) and `maos-spirit-abi`
//! (for SpiritId). It does NOT depend on `maos-kernel-core` —
//! tests under `tests/` can pull it in as a dev-dependency for
//! integration runs.

pub mod distillate_corpus; // NEW — Story 4.4 five-metric distillation gate
pub mod halt_continuity_corpus; // NEW — Story 5.2 halt-continuity corpus (AC4)
pub mod halt_corpus;
pub mod hsis_corpus;
pub mod intent_lineage_corpus; // NEW — Story 6.2 AC4 NFR-Aud-14 100% intent_lineage corpus
pub mod isolation_corpus; // NEW — Story 4.5 cross-Spirit isolation 200-corpus
pub mod onboarding_gate_corpus; // NEW — Story 7.5b NFR-Onb-1 30-min first-Spirit gate infra
pub mod retract_corpus; // NEW — Story 6.1 retract corpus (AC2)
pub mod revocation_corpus; // NEW — Story 5.4 revocation corpus (AC5)
pub mod t3_escape_corpus; // NEW — Story 5.5a T3 escape corpus (AC4)
pub mod termination_corpus; // NEW — Story 5.2 HSIS 300-corpus (AC5)
pub mod upgrade_policy_corpus; // NEW — Story 5.4 upgrade-policy corpus (AC1)

pub use distillate_corpus::{DistillateCorpus, DistillateScenario, IaaAttestation};
pub use halt_corpus::{HaltCorpus, HaltScenario, HaltScenarioOutcome};
pub use isolation_corpus::{
    CategoryAttestation, IsolationAttackCategory, IsolationCorpus, IsolationCorpusScenario,
    MethodologyAttestation,
};
pub use retract_corpus::{RetractCorpus, RetractExpectedOutcome, RetractScenario};
pub use revocation_corpus::{RevocationCorpus, RevocationExpectedOutcome, RevocationScenario};
pub use t3_escape_corpus::{
    T3AttackPayload, T3EscapeCorpus, T3EscapeScenario, T3ExpectedOutcome, T3Preconditions,
};
pub use termination_corpus::{TerminationCorpus, TerminationKind, TerminationScenario};
pub use upgrade_policy_corpus::{
    UpgradePolicyCorpus, UpgradePolicyExpectedOutcome, UpgradePolicyScenario,
};

#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("corpus directory not found: {0}")]
    NotFound(String),
    #[error("scenario parse error at {path}: {source}")]
    Parse {
        path: String,
        source: serde_json::Error,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
