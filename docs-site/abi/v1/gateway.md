<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `gateway` Module {#abi-gateway-module}

## Related {#abi-gateway-related}

- [ADR-029](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-029-gateway-submodules.md) — gateway submodule contract
- [lifecycle Module](./lifecycle) — distinct from the 14-hook `Spirit` trait


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*

Gateway Submodule trait contract — ADR-029 binding-v1.0.

**Boundary-Note:** Gateway sub-modules are **kernel-managed lifecycles**
mediated through the Capability Registry, NOT extensions to the 14-hook
`Spirit` trait. The `count_hooks!()` macro remains at 14; no new hooks
are added. This follows the CliWrapperSpirit option-(b) precedent
(Story 6.2).

A `GatewaySubmodule` implementation receives a `GatewayCtx` at `on_connect`
time. The ctx provides handles to kernel services (mailbox, capability
verification, secrets, transparency log). The implementor runs in a
`tokio::spawn`ed task owned by the `GatewayDispatcher`.


## Enums {#gateway-enums}

### `GatewayError` {#maos-spirit-abi-gateway-gatewayerror}

Error type returned by gateway submodule operations.

# Example {#maos-spirit-abi-gateway-gatewayerror-example}

```rust
use maos_spirit_abi::gateway::GatewayError;
use core::time::Duration;

let err = GatewayError::Backoff {
    retry_after: Duration::from_secs(5),
};

match err {
    GatewayError::Fatal(_) => { /* terminate */ }
    GatewayError::Backoff { retry_after } => {
        assert_eq!(retry_after, Duration::from_secs(5));
    }
    _ => { /* other error */ }
}
```


```rust
pub enum GatewayError {
    Fatal,
    Backoff,
    AuthResolveFailed,
    OutboundCapabilityDenied,
    Cancelled,
}
```


## Traits {#gateway-traits}

### `GatewaySubmodule` {#maos-spirit-abi-gateway-gatewaysubmodule}

Trait for gateway submodule implementations.

Implementors are registered with the `GatewayDispatcher` via a
`GatewaySubmoduleFactory`. The dispatcher calls `on_connect` when the
Spirit is admitted and `on_disconnect` when the Spirit is unloaded.

# Async {#maos-spirit-abi-gateway-gatewaysubmodule-async}

All methods are async. The trait uses `Pin<Box<dyn Future>>` return types
for dyn compatibility in `#![no_std]` environments.

# Example {#maos-spirit-abi-gateway-gatewaysubmodule-example}

```ignore
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
        Box::pin(async move { Ok(()) })
    }

    fn on_disconnect(
        &self, ctx: GatewayCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn on_inbound_message<'a>(
        &'a self, ctx: &'a GatewayCtx, msg: InboundMessage<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), GatewayError>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn auth_secret_ref(&self) -> &str {
        "telegram-bot-token"
    }
}
```


```rust
pub trait GatewaySubmodule {
    fn on_connect(&self, ctx: GatewayCtx) -> Pin<Box<dyn Future + Send>>;
    fn on_disconnect(&self, ctx: GatewayCtx) -> Pin<Box<dyn Future + Send>>;
    fn on_inbound_message<'a>(&'a self, ctx: &'a GatewayCtx, msg: InboundMessage<'a>) -> Pin<Box<dyn Future + Send>>;
    fn auth_secret_ref(&self) -> &str;
}
```

### `CancellationSignal` {#maos-spirit-abi-gateway-cancellationsignal}

Cancellation signal — polled by gateway implementors to detect unload.


```rust
pub trait CancellationSignal {
    fn is_cancelled(&self) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn CancellationSignal>;
}
```

### `GatewayMailboxHandle` {#maos-spirit-abi-gateway-gatewaymailboxhandle}

Mailbox handle for gateway submodules to deliver inbound IAC frames.


```rust
pub trait GatewayMailboxHandle {
    fn deliver_inbound(&self, gateway_id: &str, external_recipient_id: &str, sender_id: &str, payload: &[u8], timestamp_ns: u64) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayMailboxHandle>;
}
```

### `GatewayCapabilityHandle` {#maos-spirit-abi-gateway-gatewaycapabilityhandle}

Capability handle for verifying outbound gateway sends.


```rust
pub trait GatewayCapabilityHandle {
    fn verify_outbound(&self, token_id: [u8; 16], recipient: &str) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayCapabilityHandle>;
}
```

### `GatewaySecretsHandle` {#maos-spirit-abi-gateway-gatewaysecretshandle}

Secrets handle for resolving `auth_secret_ref`.


```rust
pub trait GatewaySecretsHandle {
    fn resolve(&self, secret_ref: &str) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewaySecretsHandle>;
}
```

### `GatewayTransparencyLogHandle` {#maos-spirit-abi-gateway-gatewaytransparencyloghandle}

Transparency log handle for writing gateway lifecycle records.


```rust
pub trait GatewayTransparencyLogHandle {
    fn write_inbound(&self, receiving_spirit_id: &str, gateway_id: &str, gateway_type: &str, external_recipient_id: &str, sender_id: &str, payload_redacted_len: u32, timestamp_ns: u64) -> Pin<Box<dyn Future + Send>>;
    fn write_outbound(&self, sending_spirit_id: &str, gateway_id: &str, gateway_type: &str, external_recipient_id: &str, cap_token_id: [u8; 16], payload_redacted_len: u32, timestamp_ns: u64, send_outcome: &str) -> Pin<Box<dyn Future + Send>>;
    fn write_lifecycle(&self, spirit_id: &str, gateway_id: &str, gateway_type: &str, event: &str, timestamp_ns: u64) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayTransparencyLogHandle>;
}
```
