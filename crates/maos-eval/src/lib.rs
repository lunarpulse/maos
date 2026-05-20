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

pub mod halt_corpus;
pub mod termination_corpus;
pub mod distillate_corpus;  // NEW — Story 4.4 five-metric distillation gate
pub mod isolation_corpus;   // NEW — Story 4.5 cross-Spirit isolation 200-corpus

pub use halt_corpus::{HaltCorpus, HaltScenario, HaltScenarioOutcome};
pub use termination_corpus::{TerminationCorpus, TerminationScenario, TerminationKind};
pub use distillate_corpus::{DistillateCorpus, DistillateScenario, IaaAttestation};
pub use isolation_corpus::{
    IsolationCorpus, IsolationCorpusScenario, IsolationAttackCategory,
    MethodologyAttestation, CategoryAttestation,
};

#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("corpus directory not found: {0}")]
    NotFound(String),
    #[error("scenario parse error at {path}: {source}")]
    Parse { path: String, source: serde_json::Error },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
