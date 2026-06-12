//! A2A router port trait per ADR-003 + ADR-012.
//!
//! Per ADR-010 hexagonal layering: the trait lives in `maos-domain` (port);
//! the concrete impl lives in `maos-a2a` (adapter). The kernel-core code
//! (`Mailbox::deliver`) holds an `Option<Arc<dyn A2ARouter>>` and routes
//! `host_id.is_some()` frames through it.
//!
//! At composition-root the daemon installs the router; absence means
//! cross-host frames fire `IacBusError::CrossHostNotConfigured`.

use crate::frame::IacFrame;
use crate::iac_bus_types::IacBusError;
use maos_spirit_abi::identity::HostId;

/// A2A router — the bridge between the same-Host IAC bus and the cross-Host
/// (or loopback) A2A transport.
///
/// Per ADR-010 the kernel-core code calls this trait through a
/// dyn-dispatched `Arc<dyn A2ARouter>`. The concrete impl in `maos-a2a`
/// implements the FR23a (loopback) + FR23b (cross-Host) protocol surfaces.
#[async_trait::async_trait]
pub trait A2ARouter: Send + Sync {
    /// Outbound: deliver this frame to the named peer Host via the configured
    /// transport.
    ///
    /// Validation order per architecture §7.3.2 + ADR-012:
    ///   1. ADR-012 send_allowlist check
    ///   2. TOFU pin verify
    ///   3. JSON-RPC framing + send + await ACK/NACK
    ///
    /// Returns `Ok(())` on receiver ACK; `Err(IacBusError::CrossHostRouteFailure)`
    /// on transport / consent / pin / partition failure. The kernel-core code
    /// surfaces the error to the caller; the application layer (Spirit)
    /// decides retry/escalate/halt per architecture §7.2.
    async fn route_outbound(&self, frame: IacFrame, peer: &HostId) -> Result<(), IacBusError>;
}
