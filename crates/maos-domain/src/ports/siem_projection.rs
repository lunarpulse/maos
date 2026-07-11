//! SIEM projection port for read-only Transparency Log export (Story 11.4c,
//! ADR-051 / NFR-Aud-11).
//!
//! Domain owns only the object-safe seam. Concrete audit-entry projection and
//! sink transport live in the out-of-kernel `maos-siem` adapter crate.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SiemProjectionError {
    #[error("SIEM projection failed: {0}")]
    Projection(String),
    #[error("SIEM sink unavailable: {0}")]
    SinkUnavailable(String),
}

pub trait SiemProjectionPort: Send + Sync {
    /// Class: data-movement
    ///
    /// Project one already-redacted audit entry representation into a SIEM-ready
    /// transport frame. Implementations must not perform authorization or mutate
    /// kernel state.
    fn project_redacted_entry(
        &self,
        redacted_entry_json: &str,
    ) -> Result<String, SiemProjectionError>;

    /// Class: supervision
    ///
    /// Whether the projection/sink configuration is currently usable.
    fn is_healthy(&self) -> bool;
}
