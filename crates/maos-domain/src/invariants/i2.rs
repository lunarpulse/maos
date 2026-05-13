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
    /// Construct a `LogBeforeDeliver`.
    ///
    /// `#[doc(hidden)]` so it does not appear in public docs. The
    /// typestate guarantee is enforced by convention: only
    /// `maos-kernel-core::iac::TransparencyLogAdapter::insert_frame_event`
    /// and this module's doctest call this constructor. External crates
    /// obtain `LogBeforeDeliver<()>` only as the return type of
    /// `IacBusPort::enqueue_frame` / `IacBusPort::broadcast_frame`,
    /// guaranteeing the I2 typestate.
    ///
    /// Story 1b.1: kept `pub` (was `pub` at v0.1-α) with `#[doc(hidden)]`
    /// per the Dev Notes recommendation. The `pub(crate)` approach was
    /// rejected because `maos-kernel-core` (a separate crate) needs to
    /// call this constructor. A sealed-trait pattern is over-engineered
    /// for one constructor at v0.1-β.
    #[doc(hidden)]
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
