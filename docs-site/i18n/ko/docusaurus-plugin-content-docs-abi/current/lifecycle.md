---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `lifecycle` Module {#abi-lifecycle-module}

## Related {#abi-lifecycle-related}

- [ABI Stability Policy](/migrate/abi-stability) — `ABI_VERSION`가 올라가는 시점
- [ctx Module](./ctx) — 모든 훅에 전달되는 `Ctx`
- [cancellation Module](./cancellation) — 훅에 사용되는 `CancellationSignal`


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

라이프사이클 훅 트레이트 — kernel과 Spirit 사이의
인프로세스 Rust 트레이트 계약.

아키텍처 §5.3에 따르면: "Spirit ABI는
kernel과 Spirit 사이의 계약입니다. 모든 Spirit이 이를 준수합니다."

이 모듈은 FR55(원래 11에서 확장된 Story 5.2)에 따른 **14-훅 시그니처 세트**를 제공합니다. 아키텍처 §5.3의 14-훅 목록에서
연기된 1개 훅은:

| 훅 | 연기 대상 | 이유 |
|---|---|---|
| `epistemic_resolve` | Story 4.1 | Halt-프로토콜 해결 |

Story 5.1이 11개 훅의 런타임 발화를 제공했습니다. Story 5.2가
전체 디스패처 통합과 함께 핫스왑 훅(`on_swap_out`, `snapshot`, `migrate`)을 추가해
총 14개로 만들었습니다.

ADR-002(v0.1에서의 Spirit 폼): 트레이트 시그니처는
`rust-inproc`과 `subprocess` 폼 모두에 사용됩니다. `CancellationSignal`
추상화가 이를 가능하게 하는 다리입니다 — 인프로세스
Spirit은 kernel의 Tokio 런타임이 지원하는 `&dyn CancellationSignal`을 받습니다; 서브프로세스 Spirit은 와이어 프로토콜이
메시지로 전달하는 신호를 받습니다.


## Enums {#lifecycle-enums}

### `HookBudgetKey` {#maos-spirit-abi-lifecycle-hookbudgetkey}

리소스 예산 봉투 키 — `#[hook(budget = "…")]`
속성이 컴파일 타임에 이 배리언트 중 하나로 파스됩니다.

kernel은 발화 시점에 manifest의 `[budget]` 섹션을 이
키와 대조합니다. 실제 시행은 Story 5.1에 제공됩니다.


```rust
pub enum HookBudgetKey {
    ContextWindow,
    TimeCapSeconds,
    CpuMaxPct,
    MemoryMaxMb,
    FdMax,
}
```

### `MigratorError` {#maos-spirit-abi-lifecycle-migratorerror}

크로스 메이저 마이그레이션 에러 — `Spirit::migrate`가 반환.

`#[non_exhaustive]`는 향후 스토리가 ABI 범프 없이 배리언트를 추가할 수 있게 합니다.
`maos-spirit-abi`가 최소 의존성(오직 `serde`)의 `#![no_std]`이므로
수작업 Display 구현입니다.


```rust
pub enum MigratorError {
    NotImplemented,
    Malformed,
    Internal,
}
```


## Functions {#lifecycle-functions}

### `kernel_invocation_allowed` {#maos-spirit-abi-lifecycle-kernel-invocation-allowed}

manifest가 `enabled_hooks` 부분집합을 선언한 Spirit에 대해
kernel이 주어진 훅을 호출해야 하면 `true`를 반환합니다.

호출 게이트는 시그니처 수준입니다: kernel이 디스패치 시점에
이 술어를 참조합니다. 런타임 훅 호출자는 Story 5.1에 제공됩니다.

# Example {#maos-spirit-abi-lifecycle-kernel-invocation-allowed-example}

```rust
use maos_spirit_abi::lifecycle::kernel_invocation_allowed;

let enabled = &["on_load", "on_frame", "on_unload"];
assert!(kernel_invocation_allowed(enabled, "on_frame"));
assert!(!kernel_invocation_allowed(enabled, "on_idle"));
```


```rust
pub fn kernel_invocation_allowed(enabled_hooks: &[&str], hook_name: &str) -> bool
```


## Traits {#lifecycle-traits}

### `Spirit` {#maos-spirit-abi-lifecycle-spirit}

Spirit 라이프사이클 트레이트.

Spirit은 kernel로부터 라이프사이클 이벤트를 받기 위해 이 트레이트를 구현합니다. 모든 메서드는 기본 no-op 본문을 가지므로, Spirit 작성자는
관심 있는 훅만 작성합니다.

# Example {#maos-spirit-abi-lifecycle-spirit-example}

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload};
use maos_spirit_abi::ctx::Ctx;

struct MySpirit;

impl Spirit for MySpirit {
    fn on_load(&self, _ctx: &mut Ctx) {
        // Initialize resources when admitted by the kernel.
    }

    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        let _data = &payload.frame_data[..payload.frame_len];
    }

    fn on_unload(&self, _ctx: &mut Ctx) {
        // Clean up resources before removal.
    }
}
```

# Firing semantics (architecture §5.3 references) {#maos-spirit-abi-lifecycle-spirit-firingsemantics-architecture5-3references}

| 훅 | 발화 시점… | 페이로드 | 구현 시점 |
|---|---|---|---|
| `on_load` | Spirit이 어드미션되어 로드(§5.3.1) | — | Story 2.1 |
| `on_start` | Spirit이 첫 `Start` 동사 수신(§5.3.2) | — | Story 2.1 |
| `on_frame` | IAC 프레임 도착(§5.3.3) | `FramePayload` | Story 2.1 |
| `on_idle` | ≥ idle_timeout_ms 동안 프레임 없음(§5.3.4) | — | Story 2.1 |
| `on_telemetry_event` | 스칼라 탭 이벤트 발화(§5.3.5) | `TelemetryEventPayload` | Story 2.1 |
| `on_schedule` | 스케줄된 호출 발화(§5.3.6) | `SchedulePayload` | Story 2.1 |
| `on_swap_in` | 선행자 상태 도착(§5.3.7) | `SwapInPayload` | Story 2.1 |
| `on_pause` | kernel이 Spirit 일시 정지(§5.3.8) | — | Story 2.1 |
| `on_resume` | kernel이 일시 정지된 Spirit 재개(§5.3.9) | — | Story 2.1 |
| `on_unload` | Spirit이 `Unload` 동사 수신(§5.3.10) | — | Story 2.1 |
| `on_consolidate` | 배치 윈도우 닫힘(§5.3.11) | `ConsolidatePayload` | Story 2.1 |
| `on_swap_out` | Spirit이 스왑 아웃 직전(§5.3.12) | — | Story 5.2 ✅ |
| `snapshot` | 핫스왑을 위한 상태 스냅샷 생성(§5.3.13) | — (`Vec<u8>` 반환) | Story 5.2 ✅ |
| `migrate` | 크로스 메이저 마이그레이션 진입(§5.3.14) | predecessor_state `&[u8]` (`Result<Vec<u8>, MigratorError>` 반환) | Story 5.2 ✅ |

모든 훅은 취소 신호,
역량 핸들, 메일박스 핸들을 전달하는 `&mut Ctx`를 받습니다.


```rust
pub trait Spirit {
    fn on_load(&self, ctx: &mut Ctx);
    fn on_start(&self, ctx: &mut Ctx);
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>);
    fn on_idle(&self, ctx: &mut Ctx);
    fn on_telemetry_event<'a>(&self, ctx: &mut Ctx, payload: &TelemetryEventPayload<'a>);
    fn on_schedule<'a>(&self, ctx: &mut Ctx, payload: &SchedulePayload<'a>);
    fn on_swap_in<'a>(&self, ctx: &mut Ctx, payload: &SwapInPayload<'a>);
    fn on_pause(&self, ctx: &mut Ctx);
    fn on_resume(&self, ctx: &mut Ctx);
    fn on_unload(&self, ctx: &mut Ctx);
    fn on_consolidate<'a>(&self, ctx: &mut Ctx, payload: &ConsolidatePayload<'a>);
    fn on_swap_out(&self, ctx: &mut Ctx);
    fn snapshot(&self, ctx: &mut Ctx) -> alloc::vec::Vec<u8>;
    fn migrate(&self, ctx: &mut Ctx, predecessor_state: &[u8]) -> Result<alloc::vec::Vec<u8>, MigratorError>;
}
```
