#![forbid(unsafe_code)]

//! Cancellation signal trait — no_std abstraction for Spirit hook cancellation.
//!
//! ADR-002 commits to subprocess-form Spirits as the v0.1 default with
//! rust-inproc gated on measurement. Subprocess Spirits live in a different
//! process from the kernel's Tokio runtime; they cannot directly reference a
//! `tokio_util::sync::CancellationToken` instance. This trait provides an
//! abstraction the wire protocol can carry as a signal.
//!
//! The SDK side (std-aware, tokio-aware) provides `TokioCancellationSignal`
//! as the production impl; tests use `NeverCancel`.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// Trait for cancellation signals — the kernel-side bridge that lets
/// hook implementations check or await cancellation without coupling
/// to a specific runtime.
///
/// Structurally parallel to the D9 pattern from Story 1b.6:
/// one no_std wire-format abstraction, one std operational adapter.
///
/// # Example
///
/// ```ignore
/// fn on_frame(&self, ctx: &mut Ctx) {
///     if ctx.cancellation().is_cancelled() {
///         return; // bail early
///     }
///     // … do work …
/// }
/// ```
pub trait CancellationSignal {
    /// Synchronous poll: returns `true` if cancellation has been requested.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maos_spirit_abi::cancellation::{CancellationSignal, NeverCancel};
    ///
    /// let signal = NeverCancel;
    /// assert!(!signal.is_cancelled());
    /// ```
    fn is_cancelled(&self) -> bool;

    /// Async await: returns a future that resolves when cancellation is
    /// requested.
    ///
    /// Not object-safe — use `is_cancelled()` on `&dyn CancellationSignal`.
    /// Concrete implementations (e.g. `TokioCancellationSignal`) override
    /// with an efficient async wait.
    fn cancelled(&self) -> CancellationFuture<'_>
    where
        Self: Sized,
    {
        CancellationFuture { signal: self }
    }
}

/// Future returned by [`CancellationSignal::cancelled`].
///
/// **Limitation:** The default implementation polls `is_cancelled()` without
/// registering a waker. Executors that rely solely on waker notifications
/// (including Tokio) will never re-poll after the first `Pending` result,
/// causing this future to hang indefinitely if the signal is not already
/// cancelled. Use `is_cancelled()` for synchronous polling in hook
/// implementations; the async `cancelled()` method is a placeholder for
/// production adapters (`TokioCancellationSignal`) that override it with
/// an efficient runtime-aware wait.
pub struct CancellationFuture<'a> {
    signal: &'a dyn CancellationSignal,
}

impl<'a> Future for CancellationFuture<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.signal.is_cancelled() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// Reference implementation: a cancellation signal that never fires.
///
/// Useful for trait-object dispatch smoke tests and SDK-side unit tests
/// that do not require an async runtime.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::cancellation::{CancellationSignal, NeverCancel};
///
/// let signal = NeverCancel;
/// assert!(!signal.is_cancelled());
///
/// // NeverCancel is the default signal used by Ctx::mock().
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverCancel;

impl CancellationSignal for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_cancel_is_always_false() {
        let nc = NeverCancel;
        assert!(!nc.is_cancelled());
    }

    #[test]
    fn never_cancel_trait_object_dispatch() {
        let nc: &dyn CancellationSignal = &NeverCancel;
        assert!(!nc.is_cancelled());
    }

    #[test]
    fn never_cancel_is_zst() {
        assert_eq!(core::mem::size_of::<NeverCancel>(), 0);
    }
}
