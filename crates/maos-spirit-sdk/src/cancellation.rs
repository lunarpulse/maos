#![forbid(unsafe_code)]

//! Tokio-aware cancellation signal adapter.
//!
//! Wraps `tokio_util::sync::CancellationToken` to implement the
//! `maos_spirit_abi::cancellation::CancellationSignal` trait. This is
//! the production implementation used by the kernel's Tokio runtime.

use maos_spirit_abi::cancellation::CancellationSignal;

/// Production cancellation signal backed by `tokio_util::sync::CancellationToken`.
///
/// The kernel constructs one of these per Spirit and passes it (as
/// `&dyn CancellationSignal`) through `Ctx` to every hook invocation.
#[derive(Debug, Clone)]
pub struct TokioCancellationSignal(pub tokio_util::sync::CancellationToken);

impl TokioCancellationSignal {
    /// Create a new `TokioCancellationSignal` wrapping the given token.
    pub fn new(token: tokio_util::sync::CancellationToken) -> Self {
        Self(token)
    }
}

impl CancellationSignal for TokioCancellationSignal {
    fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokio_signal_not_cancelled_initially() {
        let token = tokio_util::sync::CancellationToken::new();
        let signal = TokioCancellationSignal::new(token);
        assert!(!signal.is_cancelled());
    }

    #[test]
    fn tokio_signal_detects_cancellation() {
        let token = tokio_util::sync::CancellationToken::new();
        let signal = TokioCancellationSignal::new(token.clone());
        token.cancel();
        assert!(signal.is_cancelled());
    }
}
