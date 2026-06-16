#![forbid(unsafe_code)]

//! Gateway Submodule trait contract — ADR-029 binding-v1.0.
//!
//! **Boundary-Note:** Gateway sub-modules are **kernel-managed lifecycles**
//! mediated through the Capability Registry, NOT extensions to the 14-hook
//! `Spirit` trait. The `count_hooks!()` macro remains at 14; no new hooks
//! are added. This follows the CliWrapperSpirit option-(b) precedent
//! (Story 6.2).
//!
//! A `GatewaySubmodule` implementation receives a `GatewayCtx` at `on_connect`
//! time. The ctx provides handles to kernel services (mailbox, capability
//! verification, secrets, transparency log). The implementor runs in a
//! `tokio::spawn`ed task owned by the `GatewayDispatcher`.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::time::Duration;

/// Error type returned by gateway submodule operations.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::gateway::GatewayError;
/// use core::time::Duration;
///
/// let err = GatewayError::Backoff {
///     retry_after: Duration::from_secs(5),
/// };
///
/// match err {
///     GatewayError::Fatal(_) => { /* terminate */ }
///     GatewayError::Backoff { retry_after } => {
///         assert_eq!(retry_after, Duration::from_secs(5));
///     }
///     _ => { /* other error */ }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GatewayError {
    /// Fatal error — the gateway task should terminate and the dispatcher
    /// should NOT retry.
    Fatal(String),
    /// Transient error — the dispatcher should apply exponential backoff
    /// and retry `on_connect`.
    Backoff { retry_after: Duration },
    /// Authentication/authorization resolution failed.
    AuthResolveFailed(String),
    /// Outbound capability token verification denied.
    OutboundCapabilityDenied,
    /// Gateway task was cancelled during unload.
    Cancelled,
}

/// Inbound message from an external gateway.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::gateway::InboundMessage;
///
/// let msg = InboundMessage {
///     external_recipient_id: "chat_id:123456789",
///     sender_id: "user-42",
///     payload: b"hello",
///     timestamp_ns: 1_717_000_000_000_000_000,
/// };
/// assert_eq!(msg.payload, b"hello");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundMessage<'a> {
    /// External recipient identifier (e.g., "chat_id:123456789").
    pub external_recipient_id: &'a str,
    /// External sender identifier.
    pub sender_id: &'a str,
    /// Raw message bytes; capped by max_message_bytes.
    pub payload: &'a [u8],
    /// Monotonic timestamp in nanoseconds.
    pub timestamp_ns: u64,
}

/// Per-invocation context handed to GatewaySubmodule methods.
/// Carries the gateway's principal namespace handle, the IAC bus handle,
/// the capability-token issuer surface, and the cancellation signal.
///
/// All boxed handles are cloneable (cheap Arc reference semantics) so the
/// dispatcher can construct a fresh `GatewayCtx` for `on_disconnect` without
/// requiring the spawned task to release its copy.
///
/// # Example
///
/// ```ignore
/// use maos_spirit_abi::gateway::GatewayCtx;
///
/// fn connect(ctx: GatewayCtx) {
///     println!("connecting gateway {} for spirit {}", ctx.gateway_id, ctx.spirit_id);
/// }
/// ```
pub struct GatewayCtx {
    /// Gateway entry id from the manifest.
    pub gateway_id: String,
    /// Spirit id this gateway belongs to.
    pub spirit_id: String,
    /// Principal id for namespace isolation (FR31).
    pub principal_id: String,
    /// Cancellation signal — polled by the implementor to detect unload.
    pub cancel: alloc::boxed::Box<dyn CancellationSignal>,
    /// Mailbox handle — for delivering inbound IAC frames into the bus.
    pub mailbox: alloc::boxed::Box<dyn GatewayMailboxHandle>,
    /// Capability handle — for verifying outbound sends.
    pub capability: alloc::boxed::Box<dyn GatewayCapabilityHandle>,
    /// Secrets handle — for resolving `auth_secret_ref`.
    pub secrets: alloc::boxed::Box<dyn GatewaySecretsHandle>,
    /// Transparency log handle — for writing gateway lifecycle records.
    pub transparency_log: alloc::boxed::Box<dyn GatewayTransparencyLogHandle>,
}

/// Trait for gateway submodule implementations.
///
/// Implementors are registered with the `GatewayDispatcher` via a
/// `GatewaySubmoduleFactory`. The dispatcher calls `on_connect` when the
/// Spirit is admitted and `on_disconnect` when the Spirit is unloaded.
///
/// # Async
///
/// All methods are async. The trait uses `Pin<Box<dyn Future>>` return types
/// for dyn compatibility in `#![no_std]` environments.
///
/// # Example
///
/// ```ignore
/// use maos_spirit_abi::gateway::{
///     GatewaySubmodule, GatewayCtx, GatewayError, InboundMessage,
/// };
/// use core::future::Future;
/// use core::pin::Pin;
/// use alloc::boxed::Box;
///
/// struct TelegramGateway;
///
/// impl GatewaySubmodule for TelegramGateway {
///     fn on_connect(
///         &self, ctx: GatewayCtx,
///     ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>> {
///         Box::pin(async move { Ok(()) })
///     }
///
///     fn on_disconnect(
///         &self, ctx: GatewayCtx,
///     ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>> {
///         Box::pin(async move { Ok(()) })
///     }
///
///     fn on_inbound_message<'a>(
///         &'a self, ctx: &'a GatewayCtx, msg: InboundMessage<'a>,
///     ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send + 'a>> {
///         Box::pin(async move { Ok(()) })
///     }
///
///     fn auth_secret_ref(&self) -> &str {
///         "telegram-bot-token"
///     }
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait GatewaySubmodule: Send + Sync {
    /// Establish the long-lived connection. Fires at Spirit-admission
    /// time AFTER cap-tokens are issued.
    fn on_connect(
        &self,
        ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;

    /// Tear down the connection cleanly. Fires at Spirit-unload time
    /// OR at gateway-fatal-error.
    fn on_disconnect(
        &self,
        ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;

    /// Fire when an external message arrives at the gateway.
    fn on_inbound_message<'a>(
        &'a self,
        ctx: &'a GatewayCtx,
        msg: InboundMessage<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send + 'a>>;

    /// Identifier of the secret-reference the kernel must resolve before
    /// `on_connect`. Implementor returns the manifest's `auth_secret_ref`
    /// verbatim; the kernel performs the keychain lookup.
    fn auth_secret_ref(&self) -> &str;
}

/// Cancellation signal — polled by gateway implementors to detect unload.
pub trait CancellationSignal: Send + Sync {
    /// Returns true when the dispatcher has requested cancellation.
    fn is_cancelled(&self) -> Pin<Box<dyn Future<Output = bool> + Send>>;
    /// Clone this handle into a new boxed trait object.
    fn clone_box(&self) -> Box<dyn CancellationSignal>;
}

impl Clone for Box<dyn CancellationSignal> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Mailbox handle for gateway submodules to deliver inbound IAC frames.
pub trait GatewayMailboxHandle: Send + Sync {
    /// Deliver an inbound frame into the IAC bus toward the Spirit's mailbox.
    fn deliver_inbound(
        &self,
        gateway_id: &str,
        external_recipient_id: &str,
        sender_id: &str,
        payload: &[u8],
        timestamp_ns: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;
    /// Clone this handle into a new boxed trait object.
    fn clone_box(&self) -> Box<dyn GatewayMailboxHandle>;
}

impl Clone for Box<dyn GatewayMailboxHandle> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Capability handle for verifying outbound gateway sends.
pub trait GatewayCapabilityHandle: Send + Sync {
    /// Verify that the Spirit holds a valid cap-token for outbound send
    /// on this gateway with the given recipient.
    fn verify_outbound(
        &self,
        token_id: [u8; 16],
        recipient: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;
    /// Clone this handle into a new boxed trait object.
    fn clone_box(&self) -> Box<dyn GatewayCapabilityHandle>;
}

impl Clone for Box<dyn GatewayCapabilityHandle> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Secrets handle for resolving `auth_secret_ref`.
pub trait GatewaySecretsHandle: Send + Sync {
    /// Resolve a secret by reference. Returns the opaque secret bytes.
    fn resolve(
        &self,
        secret_ref: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, GatewayError>> + Send>>;
    /// Clone this handle into a new boxed trait object.
    fn clone_box(&self) -> Box<dyn GatewaySecretsHandle>;
}

impl Clone for Box<dyn GatewaySecretsHandle> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Transparency log handle for writing gateway lifecycle records.
pub trait GatewayTransparencyLogHandle: Send + Sync {
    /// Write an inbound message record to the transparency log.
    fn write_inbound(
        &self,
        receiving_spirit_id: &str,
        gateway_id: &str,
        gateway_type: &str,
        external_recipient_id: &str,
        sender_id: &str,
        payload_redacted_len: u32,
        timestamp_ns: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;

    /// Write an outbound message record to the transparency log.
    fn write_outbound(
        &self,
        sending_spirit_id: &str,
        gateway_id: &str,
        gateway_type: &str,
        external_recipient_id: &str,
        cap_token_id: [u8; 16],
        payload_redacted_len: u32,
        timestamp_ns: u64,
        send_outcome: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;

    /// Write a lifecycle event (connect, disconnect, error) to the TL.
    fn write_lifecycle(
        &self,
        spirit_id: &str,
        gateway_id: &str,
        gateway_type: &str,
        event: &str,
        timestamp_ns: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;

    /// Clone this handle into a new boxed trait object.
    fn clone_box(&self) -> Box<dyn GatewayTransparencyLogHandle>;
}

impl Clone for Box<dyn GatewayTransparencyLogHandle> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
