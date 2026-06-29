---
title: 에러 처리
sidebar_position: 13
description: MAOS Spirit의 타입화된 에러와 복구 클래스.
review_status: machine
---

# 에러 처리

## Problem

Spirit이 에러를 만납니다 — 프로바이더 타임아웃, 역량 철회, 추론 호출이 무효 출력 반환, 또는 핫스왑 마이그레이션 실패. kernel이 올바른 복구 정책(재시작, 운영자 에스컬레이션, 또는 halt 트리거)을 적용할 수 있도록 kernel이 이해하는 방식으로 이들 에러를 처리해야 합니다.

## Solution

`maos-domain`과 `maos-spirit-abi`의 타입화된 에러 enum을 사용해 실패를 정확히 전달합니다:

```rust
use maos_spirit_abi::lifecycle::{Spirit, MigratorError, FramePayload};
use maos_spirit_abi::ctx::Ctx;

pub struct ResilientSpirit;

impl Spirit for ResilientSpirit {
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }

        match self.process_frame(payload) {
            Ok(()) => {}
            Err(SpiritError::Transient(msg)) => {
                // Transient errors: log and let the kernel retry via
                // the supervision watchdog. Don't panic.
                log_warning(&msg);
            }
            Err(SpiritError::Fatal(msg)) => {
                // Fatal errors: the Spirit cannot continue. Panic
                // triggers the [on_crash] policy (restart / stop).
                panic!("fatal: {msg}");
            }
            Err(SpiritError::InvalidInput(msg)) => {
                // Bad input from upstream: log and drop the frame.
                // Do NOT panic — bad input is the sender's problem.
                log_warning(&format!("dropping frame: {msg}"));
            }
        }
    }

    fn migrate(
        &self,
        _ctx: &mut Ctx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, MigratorError> {
        // Return typed migration errors the kernel can act on.
        if predecessor_state.is_empty() {
            return Err(MigratorError::DeserializationFailed(
                "empty predecessor state".into(),
            ));
        }

        let version = read_schema_version(predecessor_state);
        match version {
            Some(v) if v > 2 => Err(MigratorError::UnsupportedVersion(v)),
            Some(_) => do_migration(predecessor_state),
            None => Err(MigratorError::DeserializationFailed(
                "missing schema_version field".into(),
            )),
        }
    }
}

/// Spirit-internal error classification.
enum SpiritError {
    /// Retryable — network timeout, temporary provider unavailability.
    Transient(String),
    /// Unrecoverable — corrupt state, missing required configuration.
    Fatal(String),
    /// Bad input — malformed frame, invalid payload.
    InvalidInput(String),
}

impl ResilientSpirit {
    fn process_frame(&self, payload: &FramePayload) -> Result<(), SpiritError> {
        // Your processing logic here.
        Ok(())
    }
}

// Stubs for illustration.
fn log_warning(msg: &str) {}
fn read_schema_version(data: &[u8]) -> Option<u32> { None }
fn do_migration(data: &[u8]) -> Result<Vec<u8>, MigratorError> {
    Ok(data.to_vec())
}
```

manifest에서 복구 정책을 구성합니다:

```toml
[on_crash]
action = "restart"                # restart | stop | notify_operator

[on_revocation]
action = "graceful_shutdown"      # graceful_shutdown | immediate_stop | notify_only

[supervision]
heartbeat_interval_ms = 5000
progress_threshold_ms = 30000
silent_failure_threshold_ms = 30000
```

## Discussion

MAOS 에러 처리는 계층화된 모델을 따릅니다:

**계층 1: Spirit 내부 에러** — 코드가 에러를 일시적(재시도 안전), 치명적(panic), 무효 입력(드롭 후 로그)으로 분류합니다. kernel은 이 분류를 직접 보지 않습니다; 훅이 정상 반환했는지 panic했는지만 봅니다.

**계층 2: ABI 수준 타입화된 에러** — `MigratorError` enum은 Spirit이 kernel에 타입화된 에러를 반환하는 유일한 곳입니다. 배리언트가 kernel에 무엇이 잘못되었는지 정확히 알려줍니다:

| 배리언트 | 의미 | kernel 동작 |
|---|---|---|
| `NotImplemented` | Spirit이 마이그레이션을 지원하지 않음 | 클린 시작(상태 전달 없음) |
| `UnsupportedVersion(u32)` | 선행 버전이 너무 오래됨/새로움 | 진단과 함께 어드미션 실패 |
| `DeserializationFailed(String)` | 상태 블롭이 손상되거나 읽을 수 없음 | 스왑 실패, transparency log에 기록 |
| `SerializationFailed(String)` | 새 상태를 인코딩할 수 없음 | 스왑 실패, transparency log에 기록 |

**계층 3: kernel 복구 정책** — `[on_crash]`와 `[on_revocation]` manifest 섹션이 Spirit 실패 시 kernel이 하는 일을 선언합니다:

- **`restart`** — kernel이 새 상태로 Spirit을 재시작합니다(핫스왑 없음).
- **`stop`** — kernel이 Spirit을 영구 실패로 표시합니다.
- **`notify_operator`** — kernel이 알림을 보내고 사람의 개입을 기다립니다.
- **`graceful_shutdown`** — 역량 철회 시 kernel이 정지 전 `on_unload`를 발화합니다.
- **`immediate_stop`** — 철회 시 kernel이 정리 없이 즉시 정지합니다.

**모범 사례:**

- 무효 입력에서 절대 panic하지 마세요. 로그하고, 프레임을 드롭하고, 계속하세요.
- `MigratorError` 배리언트를 정확히 사용하세요 — kernel의 진단이 이에 의존합니다.
- 에러를 조용히 삼키는 Spirit을 잡기 위해 `silent_failure_threshold_ms`를 설정하세요. Spirit이 이 기간 동안 프레임을 처리하지도 panic하지도 않으면 kernel이 에스컬레이션합니다.
- `/errors/`의 에러 카탈로그가 kernel 에러 코드를 사람이 읽을 수 있는 설명과 해결 단계로 매핑합니다.
