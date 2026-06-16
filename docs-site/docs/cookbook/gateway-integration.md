---
title: Gateway Integration
sidebar_position: 9
description: Implementing a GatewaySubmodule for Telegram, Slack, or Discord integration.
---

# Gateway Integration

## Problem

Your Spirit needs to receive messages from an external chat platform (Slack, Telegram, Discord) and deliver responses back. MAOS gateways run as kernel-managed lifecycle submodules — you implement the `GatewaySubmodule` trait, and the kernel handles connection lifecycle, secret resolution, and transparency logging.

## Solution

Declare a gateway entry in the manifest:

```toml
[[gateway]]
id = "slack-bot"
type = "slack"
auth_secret_ref = "vault://slack-bot-token"
on_inbound = "on_frame"
reconnect_backoff_secs = 5
max_message_bytes = 4096
```

Implement the `GatewaySubmodule` trait:

```rust
use maos_spirit_abi::gateway::{
    GatewaySubmodule, GatewayCtx, GatewayError, InboundMessage,
};
use core::future::Future;
use core::pin::Pin;
use alloc::boxed::Box;

pub struct SlackGateway {
    auth_secret_ref: String,
}

impl SlackGateway {
    pub fn new(auth_secret_ref: &str) -> Self {
        Self {
            auth_secret_ref: auth_secret_ref.into(),
        }
    }
}

impl GatewaySubmodule for SlackGateway {
    /// Called at Spirit-admission time after cap-tokens are issued.
    /// Establish the long-lived WebSocket connection to Slack.
    fn on_connect(
        &self,
        ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async move {
            // Resolve the bot token from the secrets handle.
            let token = ctx.secrets()
                .resolve("vault://slack-bot-token")
                .await
                .map_err(|e| GatewayError::AuthFailed(e.to_string().into()))?;

            // Open WebSocket, register event handlers...
            // The GatewayCtx carries a CancellationSignal — poll it
            // in your event loop to detect kernel-initiated teardown.

            Ok(())
        })
    }

    /// Called at Spirit-unload or on fatal gateway error.
    fn on_disconnect(
        &self,
        ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async move {
            // Close the WebSocket connection gracefully.
            // Log the disconnect to the transparency log.
            ctx.transparency_log()
                .write_disconnect("slack-bot", "clean shutdown")
                .await
                .map_err(|e| GatewayError::TransparencyLogFailed(
                    e.to_string().into(),
                ))?;

            Ok(())
        })
    }

    /// Called when a message arrives from the external platform.
    fn on_inbound_message<'a>(
        &'a self,
        ctx: &'a GatewayCtx,
        msg: InboundMessage<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send + 'a>> {
        Box::pin(async move {
            // Deliver the inbound message as an IAC frame to the Spirit.
            ctx.mailbox()
                .deliver_inbound(msg.channel_id, msg.payload)
                .await
                .map_err(|e| GatewayError::DeliveryFailed(
                    e.to_string().into(),
                ))?;

            Ok(())
        })
    }

    /// Return the manifest's auth_secret_ref so the kernel can
    /// resolve the secret before calling on_connect.
    fn auth_secret_ref(&self) -> &str {
        &self.auth_secret_ref
    }
}
```

## Discussion

Gateways (Story 6.5 / FR54 / ADR-029) are **kernel-managed lifecycles**, not Spirit-trait hooks. The distinction matters:

- The `GatewaySubmodule` trait is separate from the 14-hook `Spirit` trait. A Spirit can implement both.
- The gateway runs in a `tokio::spawn`ed task owned by the `GatewayDispatcher`. It has its own `GatewayCtx` with handles to the mailbox, capability registry, secrets store, and transparency log.
- Inbound messages are routed to the Spirit via the `on_inbound` field — currently only `on_frame` is supported, delivering messages as `FrameKind::GatewayInbound` through the existing frame dispatch.

**Supported gateway types** at v0.5: `slack`, `telegram`, `discord`, `webhook`.

**The `GatewayCtx` handles:**

| Handle | Purpose |
|---|---|
| `secrets()` | Resolve `auth_secret_ref` tokens from the keychain |
| `mailbox()` | Deliver inbound IAC frames to the Spirit |
| `capability()` | Verify capability tokens for outbound sends |
| `transparency_log()` | Write lifecycle records (connect, disconnect, errors) |
| `cancellation()` | Poll for kernel-initiated teardown |

All handles are `Clone` (cheap `Arc` semantics) so the dispatcher can construct a fresh `GatewayCtx` for `on_disconnect` without requiring the spawned task to release its copy.

**Reconnection:** The kernel handles reconnection backoff using `reconnect_backoff_secs` from the manifest. If `on_connect` returns an error, the kernel waits and retries. If the error is `GatewayError::AuthFailed`, the kernel does not retry — it unloads the Spirit.
