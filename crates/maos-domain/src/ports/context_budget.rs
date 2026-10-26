#![forbid(unsafe_code)]

//! Context-token budget port (Story 18-5, ADR-016).
//!
//! A Spirit reports its own context occupancy; the kernel never estimates
//! tokens. The root builds one implementation per Spirit, fixed to its
//! admitted pid and to its manifest window and thresholds, so a Spirit can
//! neither report for another pid nor choose its own window.

/// Occupancy of a Spirit's self-reported context against its manifest window.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextLevel {
    Within,
    Pressure,
    Limit,
    Exhausted,
}

/// The synchronous answer to a report: the level now and the highest level
/// crossed upward by this report (`None` when no threshold was crossed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextStatus {
    pub used: u32,
    pub window: u32,
    pub level: ContextLevel,
    pub crossed: Option<ContextLevel>,
}

/// A report the kernel cannot attribute.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContextReportError {
    /// The bound pid is not (or no longer) admitted.
    #[error("context report refused: the Spirit is not admitted")]
    NotAdmitted,
}

/// The Spirit-facing context-token report port. `Send + Sync` because a
/// Spirit holds it as `Arc<dyn ContextBudgetPort>`.
pub trait ContextBudgetPort: Send + Sync {
    /// Class: universal-arithmetic
    ///
    /// Record `used_tokens` as the Spirit's current context occupancy (the
    /// last report wins) and return the resulting typed status.
    fn report_context_used(&self, used_tokens: u32) -> Result<ContextStatus, ContextReportError>;
}
