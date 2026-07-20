---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `gateway` Module {#abi-gateway-module}

## Related {#abi-gateway-related}

- [ADR-029](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-029-gateway-submodules.md) — 게이트웨이 서브모듈 계약
- [lifecycle Module](./lifecycle) — 14-훅 `Spirit` 트레이트와 구별


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

Gateway Submodule 트레이트 계약 — ADR-029 binding-v1.0.

**경계 노트(Boundary-Note):** 게이트웨이 서브모듈은 Capability Registry를 통해 중재되는 **kernel이 관리하는 라이프사이클**이지, 14-훅 `Spirit` 트레이트의 확장이 아닙니다. `count_hooks!()` 매크로는 14로 유지됩니다; 새 훅은 추가되지 않습니다. 이는 CliWrapperSpirit option-(b) 전례
(Story 6.2)를 따릅니다.

`GatewaySubmodule` 구현은 `on_connect`
시점에 `GatewayCtx`를 받습니다. ctx는 kernel 서비스(메일박스, 역량
검증, 시크릿, transparency log)에 대한 핸들을 제공합니다. 구현자는
`GatewayDispatcher`가 소유한 `tokio::spawn`된 태스크에서 실행됩니다.


## Enums {#gateway-enums}

### `GatewayError` {#maos-spirit-abi-gateway-gatewayerror}

게이트웨이 서브모듈 연산이 반환하는 에러 타입.

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

게이트웨이 서브모듈 구현용 트레이트.

구현자는
`GatewaySubmoduleFactory`를 통해 `GatewayDispatcher`에 등록됩니다. 디스패처는
Spirit이 어드미션되면 `on_connect`를, Spirit이 언로드되면 `on_disconnect`를 호출합니다.

# Async {#maos-spirit-abi-gateway-gatewaysubmodule-async}

모든 메서드는 비동기입니다. 이 트레이트는 `#![no_std]` 환경에서
dyn 호환성을 위해 `Pin<Box<dyn Future>>` 반환 타입을 사용합니다.

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

취소 신호 — 언로드 감지를 위해 게이트웨이 구현자가 폴링.


```rust
pub trait CancellationSignal {
    fn is_cancelled(&self) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn CancellationSignal>;
}
```

### `GatewayMailboxHandle` {#maos-spirit-abi-gateway-gatewaymailboxhandle}

인바운드 IAC 프레임을 전달하기 위한 게이트웨이 서브모듈용 메일박스 핸들.


```rust
pub trait GatewayMailboxHandle {
    fn deliver_inbound(&self, gateway_id: &str, external_recipient_id: &str, sender_id: &str, payload: &[u8], timestamp_ns: u64) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayMailboxHandle>;
}
```

### `GatewayCapabilityHandle` {#maos-spirit-abi-gateway-gatewaycapabilityhandle}

아웃바운드 게이트웨이 전송 검증용 역량 핸들.


```rust
pub trait GatewayCapabilityHandle {
    fn verify_outbound(&self, token_id: [u8; 16], recipient: &str) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayCapabilityHandle>;
}
```

### `GatewaySecretsHandle` {#maos-spirit-abi-gateway-gatewaysecretshandle}

`auth_secret_ref` 해결용 시크릿 핸들.


```rust
pub trait GatewaySecretsHandle {
    fn resolve(&self, secret_ref: &str) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewaySecretsHandle>;
}
```

### `GatewayTransparencyLogHandle` {#maos-spirit-abi-gateway-gatewaytransparencyloghandle}

게이트웨이 라이프사이클 레코드를 작성하기 위한 transparency log 핸들.


```rust
pub trait GatewayTransparencyLogHandle {
    fn write_inbound(&self, receiving_spirit_id: &str, gateway_id: &str, gateway_type: &str, external_recipient_id: &str, sender_id: &str, payload_redacted_len: u32, timestamp_ns: u64) -> Pin<Box<dyn Future + Send>>;
    fn write_outbound(&self, sending_spirit_id: &str, gateway_id: &str, gateway_type: &str, external_recipient_id: &str, cap_token_id: [u8; 16], payload_redacted_len: u32, timestamp_ns: u64, send_outcome: &str) -> Pin<Box<dyn Future + Send>>;
    fn write_lifecycle(&self, spirit_id: &str, gateway_id: &str, gateway_type: &str, event: &str, timestamp_ns: u64) -> Pin<Box<dyn Future + Send>>;
    fn clone_box(&self) -> Box<dyn GatewayTransparencyLogHandle>;
}
```
