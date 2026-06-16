---
title: gateway
sidebar_position: 7
description: "GatewaySubmodule trait and GatewayCtx — external messaging gateway contract per ADR-029."
---

# `gateway` Module

The gateway module defines the trait contract for external messaging gateway submodules (Telegram, Slack, Discord, Signal, Email). Gateway submodules are **kernel-managed lifecycles** mediated through the Capability Registry — they are NOT extensions to the 14-hook `Spirit` trait.

Introduced in Story 6.5 per ADR-029 binding-v1.0.

## GatewaySubmodule Trait

The core trait for gateway implementations. Implementors are registered with the `GatewayDispatcher` via a `GatewaySubmoduleFactory`. All methods are async, using `Pin<Box<dyn Future>>` return types for dyn compatibility in `#![no_std]`.

```rust
pub trait GatewaySubmodule: Send + Sync {
    fn on_connect(
        &self, ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;

    fn on_disconnect(
        &self, ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;

    fn on_inbound_message<'a>(
        &'a self, ctx: &'a GatewayCtx, msg: InboundMessage<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send + 'a>>;

    fn auth_secret_ref(&self) -> &str;
}
```

### Methods

| Method | Fires When | Description |
|---|---|---|
| `on_connect` | Spirit admission, after cap-tokens are issued | Establish the long-lived external connection |
| `on_disconnect` | Spirit unload or gateway-fatal-error | Tear down the connection cleanly |
| `on_inbound_message` | External message arrives at the gateway | Process the inbound message |
| `auth_secret_ref` | Before `on_connect` | Return the manifest's `auth_secret_ref` for kernel keychain lookup |

### Example: Implementing a Gateway

```rust
use maos_spirit_abi::gateway::{
    GatewaySubmodule, GatewayCtx, GatewayError, InboundMessage,
};
use core::future::Future;
use core::pin::Pin;
use alloc::boxed::Box;

struct TelegramGateway;

impl GatewaySubmodule for TelegramGateway {
    fn on_connect(
        &self, ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async move {
            // Establish Telegram bot connection using resolved secret
            // ctx.secrets, ctx.mailbox, ctx.cancel are available
            Ok(())
        })
    }

    fn on_disconnect(
        &self, ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async move {
            // Clean shutdown of Telegram polling
            Ok(())
        })
    }

    fn on_inbound_message<'a>(
        &'a self, ctx: &'a GatewayCtx, msg: InboundMessage<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send + 'a>> {
        Box::pin(async move {
            // Deliver message to Spirit via IAC bus
            ctx.mailbox.deliver_inbound(
                &ctx.gateway_id,
                msg.external_recipient_id,
                msg.sender_id,
                msg.payload,
                msg.timestamp_ns,
            ).await
        })
    }

    fn auth_secret_ref(&self) -> &str {
        "telegram-bot-token"
    }
}
```

## GatewayCtx

Per-invocation context handed to `GatewaySubmodule` methods. Carries handles to kernel services — all boxed handles are cloneable (cheap `Arc` semantics).

```rust
pub struct GatewayCtx {
    pub gateway_id: String,
    pub spirit_id: String,
    pub principal_id: String,
    pub cancel: Box<dyn CancellationSignal>,
    pub mailbox: Box<dyn GatewayMailboxHandle>,
    pub capability: Box<dyn GatewayCapabilityHandle>,
    pub secrets: Box<dyn GatewaySecretsHandle>,
    pub transparency_log: Box<dyn GatewayTransparencyLogHandle>,
}
```

| Field | Type | Purpose |
|---|---|---|
| `gateway_id` | `String` | Gateway entry id from the manifest |
| `spirit_id` | `String` | Spirit id this gateway belongs to |
| `principal_id` | `String` | Principal id for namespace isolation (FR31) |
| `cancel` | `Box<dyn CancellationSignal>` | Poll for unload cancellation |
| `mailbox` | `Box<dyn GatewayMailboxHandle>` | Deliver inbound IAC frames |
| `capability` | `Box<dyn GatewayCapabilityHandle>` | Verify outbound sends |
| `secrets` | `Box<dyn GatewaySecretsHandle>` | Resolve `auth_secret_ref` |
| `transparency_log` | `Box<dyn GatewayTransparencyLogHandle>` | Write lifecycle records |

## GatewayError

Error type returned by gateway submodule operations.

```rust
#[non_exhaustive]
pub enum GatewayError {
    Fatal(String),                      // Terminal — no retry
    Backoff { retry_after: Duration },  // Transient — dispatcher retries with backoff
    AuthResolveFailed(String),          // Secret resolution failed
    OutboundCapabilityDenied,           // Cap-token verification denied
    Cancelled,                          // Gateway task cancelled during unload
}
```

### Example: Error Handling

```rust
use maos_spirit_abi::gateway::GatewayError;
use core::time::Duration;

fn handle_connect_error(err: GatewayError) {
    match err {
        GatewayError::Fatal(msg) => {
            // Log and terminate — dispatcher will NOT retry
        }
        GatewayError::Backoff { retry_after } => {
            // Dispatcher applies exponential backoff and retries on_connect
        }
        GatewayError::AuthResolveFailed(msg) => {
            // Secret reference could not be resolved from keychain
        }
        GatewayError::OutboundCapabilityDenied => {
            // Spirit lacks Scope::GatewaySend cap-token
        }
        GatewayError::Cancelled => {
            // Normal unload path
        }
    }
}
```

## InboundMessage

Inbound message from an external gateway.

```rust
pub struct InboundMessage<'a> {
    pub external_recipient_id: &'a str,
    pub sender_id: &'a str,
    pub payload: &'a [u8],
    pub timestamp_ns: u64,
}
```

## Service Handle Traits

The gateway module provides five trait-object-safe service handles, each with `clone_box()` for cheap cloning:

### GatewayMailboxHandle

Delivers inbound IAC frames into the bus toward the Spirit's mailbox.

```rust
pub trait GatewayMailboxHandle: Send + Sync {
    fn deliver_inbound(
        &self, gateway_id: &str, external_recipient_id: &str,
        sender_id: &str, payload: &[u8], timestamp_ns: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayMailboxHandle>;
}
```

### GatewayCapabilityHandle

Verifies outbound gateway sends against cap-tokens.

```rust
pub trait GatewayCapabilityHandle: Send + Sync {
    fn verify_outbound(
        &self, token_id: [u8; 16], recipient: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayCapabilityHandle>;
}
```

### GatewaySecretsHandle

Resolves secret references (e.g., API tokens) from the kernel keychain.

```rust
pub trait GatewaySecretsHandle: Send + Sync {
    fn resolve(
        &self, secret_ref: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, GatewayError>> + Send>>;
    fn clone_box(&self) -> Box<dyn GatewaySecretsHandle>;
}
```

### GatewayTransparencyLogHandle

Writes gateway lifecycle records to the append-only transparency log.

```rust
pub trait GatewayTransparencyLogHandle: Send + Sync {
    fn write_inbound(&self, ...) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;
    fn write_outbound(&self, ...) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;
    fn write_lifecycle(&self, ...) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayTransparencyLogHandle>;
}
```

### CancellationSignal (Gateway-specific)

Gateway-specific cancellation signal (distinct from the `cancellation` module's trait).

```rust
pub trait CancellationSignal: Send + Sync {
    fn is_cancelled(&self) -> Pin<Box<dyn Future<Output = bool> + Send>>;
    fn clone_box(&self) -> Box<dyn CancellationSignal>;
}
```

Note: This is a separate trait from `cancellation::CancellationSignal`. The gateway variant uses async `is_cancelled()` returning a boxed future, while the `cancellation` module's trait uses synchronous `is_cancelled() -> bool`.
