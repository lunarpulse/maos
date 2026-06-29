---
title: 게이트웨이 통합
sidebar_position: 9
description: Telegram, Slack, Discord 통합을 위한 GatewaySubmodule 구현.
review_status: machine
---

# 게이트웨이 통합

## Problem

Spirit이 외부 채팅 플랫폼(Slack, Telegram, Discord)에서 메시지를 받고 응답을 다시 전달해야 합니다. MAOS 게이트웨이는 kernel이 관리하는 라이프사이클 서브모듈로 실행됩니다 — `GatewaySubmodule` 트레이트를 구현하면, kernel이 연결 라이프사이클, 시크릿 해결, transparency 로깅을 처리합니다.

## Solution

manifest에 게이트웨이 항목을 선언합니다:

```toml
[[gateway]]
id = "slack-bot"
type = "slack"
auth_secret_ref = "vault://slack-bot-token"
on_inbound = "on_frame"
reconnect_backoff_secs = 5
max_message_bytes = 4096
```

`GatewaySubmodule` 트레이트를 구현합니다:

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

게이트웨이(Story 6.5 / FR54 / ADR-029)는 **kernel이 관리하는 라이프사이클**이지, Spirit 트레이트 훅이 아닙니다. 이 구분이 중요합니다:

- `GatewaySubmodule` 트레이트는 14-훅 `Spirit` 트레이트와 별개입니다. Spirit이 둘 다 구현할 수 있습니다.
- 게이트웨이는 `GatewayDispatcher`가 소유한 `tokio::spawn`된 태스크에서 실행됩니다. 메일박스, 역량 레지스트리, 시크릿 스토어, transparency log에 대한 핸들을 가진 자체 `GatewayCtx`가 있습니다.
- 인바운드 메시지는 `on_inbound` 필드를 통해 Spirit으로 라우팅됩니다 — 현재 `on_frame`만 지원되며, 기존 프레임 디스패치를 통해 메시지를 `FrameKind::GatewayInbound`로 전달합니다.

v0.5의 **지원 게이트웨이 타입**: `slack`, `telegram`, `discord`, `webhook`.

**`GatewayCtx` 핸들:**

| 핸들 | 목적 |
|---|---|
| `secrets()` | 키체인에서 `auth_secret_ref` 토큰 해결 |
| `mailbox()` | Spirit에게 인바운드 IAC 프레임 전달 |
| `capability()` | 아웃바운드 전송을 위한 역량 토큰 검증 |
| `transparency_log()` | 라이프사이클 레코드 작성(연결, 해제, 에러) |
| `cancellation()` | kernel이 시작한 해제 감지를 위해 폴링 |

모든 핸들은 `Clone`(저렴한 `Arc` 의미)이므로 디스패처가 스폰된 태스크가 자신의 사본을 해제하도록 요구하지 않고 `on_disconnect`를 위한 새 `GatewayCtx`를 생성할 수 있습니다.

**재연결:** kernel이 manifest의 `reconnect_backoff_secs`를 사용해 재연결 백오프를 처리합니다. `on_connect`가 에러를 반환하면 kernel이 대기 후 재시도합니다. 에러가 `GatewayError::AuthFailed`이면 kernel은 재시도하지 않고 — Spirit을 언로드합니다.
