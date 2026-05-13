//! I2: Every IAC interaction is logged before delivery.
//!
//! No ACK/NACK has ever been sent without an entry in the Transparency Log.
//! The "no invisible actions" rule. Without this, peer trust collapses.
//!
//! # Enforcement
//!
//! - **v0.1**: `runtime` — IAC Bus writes log before it writes mailbox.
//! - **v0.3 / v0.5 / v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! The `LogBeforeDeliver<T>` typestate wrapper codifies I2: construction
//! implies the inner payload has been written to the Transparency Log
//! before delivery.
//!
//! ```
//! use maos_domain::invariants::i2::{InvariantI2, LogBeforeDeliver};
//!
//! let _marker: InvariantI2 = InvariantI2;
//! // LogBeforeDeliver wraps a payload that has already been logged.
//! let payload: LogBeforeDeliver<&str> = LogBeforeDeliver::new("hello");
//! assert_eq!(payload.into_inner(), "hello");
//! ```

/// I2 marker type — Every IAC interaction is logged before delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI2;

/// Typestate wrapper: construction implies the inner payload has been
/// written to the Transparency Log before delivery.
///
/// The kernel's IAC Bus is the only constructor at runtime; Spirits
/// cannot construct this wrapper directly.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogBeforeDeliver<T> {
    inner: T,
}

impl<T> LogBeforeDeliver<T> {
    /// Construct a `LogBeforeDeliver`. At v0.1-α this is `pub` so the
    /// doctest compiles; runtime enforcement restricts construction to
    /// the kernel's IAC Bus in Story 1b.2.
    // TODO(v0.1-α): pub for doctest; Story 1b.2 restricts to kernel via
    // pub(crate) or sealed trait pattern.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Consume the wrapper and return the logged payload.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_before_deliver_roundtrip() {
        let payload = LogBeforeDeliver::new(42);
        assert_eq!(payload.into_inner(), 42);
    }
}
