#![forbid(unsafe_code)]

//! Echo Gateway Submodule — reference fixture for testing.
//!
//! Story 6.5 / FR54. A no-op gateway that accepts all inbound messages
//! and verifies the ctx handles are callable. Used in integration tests
//! and the smoke arm.

use maos_spirit_abi::gateway::{GatewayCtx, GatewayError, GatewaySubmodule, InboundMessage};

/// Echo gateway — accepts all messages, no external I/O.
pub struct EchoGatewaySubmodule;

impl GatewaySubmodule for EchoGatewaySubmodule {
    fn on_connect(
        &self,
        ctx: GatewayCtx,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async move {
            ctx.mailbox
                .deliver_inbound(&ctx.gateway_id, "", "", &[0x01], 0)
                .await?;
            ctx.capability.verify_outbound([0u8; 16], "").await?;
            let _ = ctx.secrets.resolve("test").await?;
            ctx.transparency_log
                .write_lifecycle(&ctx.spirit_id, &ctx.gateway_id, "echo", "connect", 0)
                .await?;
            Ok(())
        })
    }

    fn on_disconnect(
        &self,
        ctx: GatewayCtx,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async move {
            ctx.transparency_log
                .write_lifecycle(&ctx.spirit_id, &ctx.gateway_id, "echo", "disconnect", 0)
                .await?;
            Ok(())
        })
    }

    fn on_inbound_message<'a>(
        &'a self,
        ctx: &'a GatewayCtx,
        msg: InboundMessage<'a>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send + 'a>>
    {
        Box::pin(async move {
            ctx.mailbox
                .deliver_inbound(
                    &ctx.gateway_id,
                    msg.external_recipient_id,
                    msg.sender_id,
                    msg.payload,
                    msg.timestamp_ns,
                )
                .await?;
            ctx.transparency_log
                .write_inbound(
                    &ctx.spirit_id,
                    &ctx.gateway_id,
                    "echo",
                    msg.external_recipient_id,
                    msg.sender_id,
                    msg.payload.len() as u32,
                    msg.timestamp_ns,
                )
                .await?;
            Ok(())
        })
    }

    fn auth_secret_ref(&self) -> &str {
        "secret:echo:default"
    }
}

/// Factory for creating EchoGatewaySubmodule instances.
pub struct EchoGatewayFactory;

impl crate::orchestrator::gateway_dispatcher::GatewaySubmoduleFactory for EchoGatewayFactory {
    fn create(
        &self,
        _entry: &maos_manifest::GatewayEntry,
    ) -> Result<Box<dyn maos_spirit_abi::gateway::GatewaySubmodule>, GatewayError> {
        Ok(Box::new(EchoGatewaySubmodule))
    }
}
