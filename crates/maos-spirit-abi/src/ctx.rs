#![forbid(unsafe_code)]

//! Spirit-author-facing context type.
//!
//! `Ctx` carries the cancellation signal, capability handle, and
//! mailbox handle a Spirit hook receives at invocation time. It is the
//! ONLY surface through which a hook can interact with the kernel —
//! per I1, Spirits cannot bypass the Capability Registry.

use crate::cancellation::CancellationSignal;

/// Opaque handle to a capability token held kernel-side.
///
/// The Spirit sees only this integer handle; the kernel resolves it
/// to the actual `CapabilityToken` at mediation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityHandle(pub u64);

/// Opaque handle to the Spirit's mailbox.
///
/// The Spirit sees only this integer handle; the kernel resolves it
/// to the actual mailbox queue at dispatch time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxHandle(pub u64);

/// Spirit-author-facing context passed to every hook.
///
/// Carries a cancellation signal (so long-running hooks can bail early),
/// an opaque capability handle (so hooks can use capability APIs
/// via the SDK), and an opaque mailbox handle (for IAC frame send/receive
/// via the SDK).
///
/// The cancellation signal is stored with a `'static` bound because
/// the kernel owns the underlying signal (e.g., an `Arc<AtomicBool>`)
/// and ensures it outlives all Spirit hook invocations. This avoids
/// lifetime parameters on the vtable dispatch functions.
#[derive(Clone, Copy)]
pub struct Ctx {
    pub(crate) cancellation: &'static dyn CancellationSignal,
    pub(crate) capability_handle: CapabilityHandle,
    pub(crate) mailbox_handle: MailboxHandle,
}

impl Ctx {
    /// Borrow the cancellation signal.
    pub fn cancellation(&self) -> &dyn CancellationSignal {
        self.cancellation
    }

    /// Opaque capability handle — the kernel resolves this at mediation time.
    pub fn capability(&self) -> CapabilityHandle {
        self.capability_handle
    }

    /// Opaque mailbox handle — the kernel resolves this at IAC dispatch time.
    pub fn mailbox(&self) -> MailboxHandle {
        self.mailbox_handle
    }
}

impl Ctx {
    /// Construct a mock `Ctx` for SDK-side unit tests.
    ///
    /// Uses `NeverCancel` + zero handles. Gated behind
    /// `#[cfg(any(test, feature = "mock"))]` so production code
    /// cannot fabricate a `Ctx`.
    #[cfg(any(test, feature = "mock"))]
    pub fn mock() -> Self {
        static NEVER: crate::cancellation::NeverCancel = crate::cancellation::NeverCancel;
        Self {
            cancellation: &NEVER,
            capability_handle: CapabilityHandle(0),
            mailbox_handle: MailboxHandle(0),
        }
    }

    /// Construct a kernel-internal `Ctx` for rust-inproc hook dispatch.
    ///
    /// Story 5.1 closure: the kernel-side `HookDispatcher` requires a
    /// `Ctx` to pass to each hook fire. At v0.3-β the rust-inproc form
    /// uses a `&'static NeverCancel` because the kernel mediates
    /// cancellation through `KernelCtx`'s SCB state, not through `Ctx`.
    /// Real handles are zero-valued (rust-inproc form does not use them;
    /// Story 5.5x's subprocess form will populate them from the wire
    /// decode handshake).
    ///
    /// This constructor is NOT gated behind the `mock` feature — it is
    /// the production-supported kernel-side surface for rust-inproc
    /// dispatch, and is callable only from within `maos-kernel-core`
    /// (no Spirit author can call this; the Spirit receives a fully-
    /// constructed `Ctx` from the kernel and never constructs one
    /// itself).
    pub fn for_rust_inproc_hook(
        capability_handle: CapabilityHandle,
        mailbox_handle: MailboxHandle,
    ) -> Self {
        static NEVER: crate::cancellation::NeverCancel = crate::cancellation::NeverCancel;
        Self {
            cancellation: &NEVER,
            capability_handle,
            mailbox_handle,
        }
    }
}

impl core::fmt::Debug for Ctx {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ctx")
            .field("cancellation", &"<dyn CancellationSignal>")
            .field("capability_handle", &self.capability_handle)
            .field("mailbox_handle", &self.mailbox_handle)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctx_mock_has_cancellation_not_cancelled() {
        let ctx = Ctx::mock();
        assert!(!ctx.cancellation().is_cancelled());
    }

    #[test]
    fn ctx_mock_has_zero_handles() {
        let ctx = Ctx::mock();
        assert_eq!(ctx.capability(), CapabilityHandle(0));
        assert_eq!(ctx.mailbox(), MailboxHandle(0));
    }

    #[test]
    fn ctx_handles_are_copy() {
        let h = CapabilityHandle(42);
        let h2 = h;
        assert_eq!(h, h2);
        assert_eq!(h.0, 42);
    }

    #[test]
    fn ctx_is_copy() {
        let ctx = Ctx::mock();
        let ctx2 = ctx;
        assert_eq!(ctx.capability(), ctx2.capability());
    }
}
