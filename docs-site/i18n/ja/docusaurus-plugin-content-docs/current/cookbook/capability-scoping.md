---
title: 역량 스코핑
sidebar_position: 6
description: kernel 서비스에 안전하게 접근하기 위해 역량 토큰을 요청하고 사용.
review_status: machine
---

# 역량 스코핑

## Problem

Spirit이 추론 프로바이더 호출, IAC 프레임 전송, 또는 MCP 도구 서버 접근이 필요합니다. MAOS는 불변량 I1을 시행합니다: Spirit은 Capability Registry를 우회할 수 없습니다. 모든 권한 있는 연산은 manifest에 선언되고 런타임에 중재되는 역량 토큰이 필요합니다.

## Solution

manifest에 필수 역량을 선언합니다:

```toml
[capabilities.required]

# Inference provider access
[capabilities.required.provider]
complete = ["anthropic/claude-3"]

# MCP tool-server access
[capabilities.required.mcp]
[[capabilities.required.mcp.servers]]
name = "search-server"
tools = ["web_search", "document_search"]
```

런타임에 `Ctx`의 `CapabilityHandle`을 사용해 kernel을 통해 호출을 중재합니다:

```rust
use maos_spirit_abi::lifecycle::Spirit;
use maos_spirit_abi::ctx::Ctx;

pub struct InferenceSpirit;

impl Spirit for InferenceSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }

        // The capability handle is an opaque u64 the kernel resolves
        // at mediation time. You never construct tokens yourself.
        let cap_handle = ctx.capability();

        // The mailbox handle lets you send/receive IAC frames —
        // also mediated through the kernel's capability registry.
        let mailbox = ctx.mailbox();

        // Use cap_handle and mailbox with the Spirit SDK's typed
        // wrappers to make inference calls, send frames, etc.
        // The kernel verifies every call against the declared scopes.
    }
}
```

## Discussion

역량 스코핑은 MAOS 불변량 I1("어떤 Spirit도 Capability Registry를 우회하지 않는다")의 메커니즘입니다. 흐름은:

1. **선언** — manifest의 `[capabilities.required]` 섹션이 Spirit에 필요한 모든 스코프를 나열합니다. 어드미션 시점에 kernel이 이를 `Scope` 값(`Scope::ProviderInfer`, `Scope::McpCall` 등)으로 변환합니다.

2. **발급** — kernel의 Capability Registry가 선언된 스코프에 대한 토큰을 발급하고 Spirit의 주체 네임스페이스에 바인딩합니다. Spirit은 불투명한 `CapabilityHandle(u64)`만 봅니다.

3. **중재** — 모든 권한 있는 SDK 호출이 핸들을 kernel에 전달하고, kernel이 이를 실제 토큰으로 해결해 스코프를 확인합니다. 선언된 스코프 밖의 호출은 `CapError`를 반환합니다.

4. **철회** — kernel은 언제든 토큰을 철회할 수 있습니다(운영자 조치, 정책 변경, Spirit 오동작). manifest의 `[on_revocation]` 섹션이 Spirit의 응답을 선언합니다.

**스코핑 원칙:**

- Spirit에 필요한 **최소** 역량 세트를 요청하세요. kernel은 모든 역량 발급을 transparency log에 기록합니다.
- 와일드카드 접근보다 **명명된 도구**를 선호하세요 — `tools` 필드를 생략하기보다 `tools = ["web_search"]`.
- `[class]`의 `trust_tier`는 발급할 수 있는 역량에 영향을 줍니다. `local` Spirit은 로컬 프로바이더에 접근할 수 있습니다; 원격 엔드포인트 접근은 `community` 또는 `audited` tier가 필요할 수 있습니다.
