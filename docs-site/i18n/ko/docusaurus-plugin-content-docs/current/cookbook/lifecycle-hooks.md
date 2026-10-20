---
title: 라이프사이클 훅
sidebar_position: 4
description: 단일 MAOS Spirit에서 여러 라이프사이클 훅 구현.
review_status: machine
---

# 라이프사이클 훅

## Problem

Spirit이 두 개 이상의 라이프사이클 이벤트에 반응해야 합니다 — 로드 시 상태 초기화, 인바운드 프레임 처리, 유휴 기간 처리, 언로드 시 정리. 전체 훅 세트와 kernel이 발화하는 순서를 알아야 합니다.

## Solution

`Spirit` 트레이트에 필요한 훅을 구현합니다. 모든 훅은 기본 no-op 본문을 가지므로, 관심 있는 것만 작성하면 됩니다:

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload, SchedulePayload};
use maos_spirit_abi::ctx::Ctx;

pub struct AnalyticsSpirit {
    event_count: std::cell::Cell<u64>,
}

impl AnalyticsSpirit {
    pub fn new() -> Self {
        Self { event_count: std::cell::Cell::new(0) }
    }
}

impl Spirit for AnalyticsSpirit {
    /// Called once when the Spirit is admitted and loaded into memory.
    fn on_load(&self, ctx: &mut Ctx) {
        // Initialise connections, load config, prepare state.
    }

    /// Called when the Spirit receives its first Start verb.
    fn on_start(&self, ctx: &mut Ctx) {
        // Begin accepting work — open listeners, start timers.
    }

    /// Called when an IAC frame arrives at the Spirit's mailbox.
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        self.event_count.set(self.event_count.get() + 1);
        // Process the inbound frame payload.
    }

    /// Called when no frames arrive for >= idle_timeout_ms.
    fn on_idle(&self, ctx: &mut Ctx) {
        // Flush buffered analytics, run compaction, emit summaries.
    }

    /// Called when the kernel pauses the Spirit (resource pressure).
    fn on_pause(&self, _ctx: &mut Ctx) {
        // Suspend background work, release optional resources.
    }

    /// Called when the kernel resumes a paused Spirit.
    fn on_resume(&self, _ctx: &mut Ctx) {
        // Re-acquire resources, restart background loops.
    }

    /// Called after the kernel has released this Spirit. State is already
    /// flipped, the receipt written, capabilities revoked, and the SCB removed.
    /// KernelCtx capability and heartbeat calls fail (ScbNotFound or denied);
    /// clean up only resources that do not require live tokens.
    fn on_unload(&self, _ctx: &mut Ctx) {
        // Flush pending writes, close connections, release resources.
    }
}
```

해제는 kernel이 소유합니다: `on_unload`는 해제 이후에 실행되는 정리 훅이며, teardown 트랜잭션이 아닙니다. 이 훅이 실행되는 동안 살아 있는 SCB, capability 토큰, heartbeat에 의존해서는 안 됩니다. Story 16-6 리뷰 2026-09-20.

kernel이 나머지를 건너뛰도록 manifest에 사용하는 훅을 선언합니다:

```toml
[lifecycle]
enabled_hooks = [
  "on_load", "on_start", "on_frame",
  "on_idle", "on_pause", "on_resume", "on_unload",
]
```

## Discussion

MAOS Spirit ABI는 14개의 라이프사이클 훅을 정의합니다. kernel은 잘 정의된 순서로 발화합니다:

| 단계 | 훅 | 시점 |
|---|---|---|
| 어드미션 | `on_load` | Spirit이 메모리에 로드됨 |
| 실행 중 | `on_start` → `on_frame` / `on_idle` / `on_schedule` / `on_telemetry_event` / `on_consolidate` | 정상 동작 |
| 일시 정지 | `on_pause` / `on_resume` | kernel이 시작한 일시 정지/재개 |
| 핫스왑 | `on_swap_out` / `snapshot` / `on_swap_in` / `migrate` | 버전 교체 |
| 해체 | `on_unload` | Spirit 제거 |

서비스 중인 `maos run` 루트(`--once` 제외)에서 호스트는 두 훅을 디스패치합니다:

- `on_telemetry_event`: 훅 펌프는 해당 루트의 모든 `scalar.tap` 쓰기(`halt.context_provided` 같은 마커 포함)를 각 발행자의 쓰기 순서대로 전달합니다. 수신 대상은 어드미션된 컴파일 내장 Spirit 중 현재 클래스가 `observer`이고 상태가 `Running`인 모든 Spirit입니다. 이벤트의 `seq`는 한 루트 안에서 고유한 저장소 쓰기 식별 레이블이며, 시계나 정렬 키가 아닙니다.
- `on_consolidate`: 어드미션된 모든 컴파일 내장 Spirit은 `Running` 상태에서 인바운드 프레임을 받지 않는 동안 자신의 `[scheduling].idle_window_ms`마다 한 번씩 빈 페이로드("지금 큐레이션하라")를 받습니다.

스폰된 형태(WASM 컴포넌트와 CLI Worker)는 두 훅 모두 받지 않습니다. 두 훅 모두 `enabled_hooks`와 격리 상태를 따릅니다.

텔레메트리 구독 권한은 manifest 필드가 아니라 운영자가 컴파일 내장 Observer 클래스를 어드미션하는 데서 나옵니다. 모든 부여/회수는 구독자의 pid에 `governance:telemetry-subscription:granted` / `governance:telemetry-subscription:revoked` 행으로 감사 기록되며, `maos audit query --spirit <id>`에서 볼 수 있습니다.

지연으로 건너뛴 이벤트 수는 집계되며 `maos: telemetry pump lagged: <n> event(s) skipped`로 보고됩니다. 연속 세 번 패닉하거나 훅 예산을 한 번 초과한 구독자는 해당 런타임 객체에 대해 격리됩니다. 핫스왑된 후속 객체는 격리 이력 없이 시작합니다.

적격성 검사와 디스패치는 원자적이지 않습니다. 디스패치와 경합하는 일시 정지나 언로드는 여전히 펜싱되지 않은 FR51(a) 공백(E16-A6)입니다.

각 디스패치는 적격성을 검사한 런타임 스냅샷을 사용합니다. 동시에 핫스왑이 일어나도 대기 중인 호출이 후속 객체로 바뀌거나 후속 객체의 실패 이력에 반영되지 않습니다. `HookPump::flush()`는 모든 발행이 멈추기를 기다리지 않고 유한한 큐 경계를 캡처합니다. 취소 시 읽지 않은 브로드캐스트 위치는 `shutdown_skipped`에 집계됩니다.

Observer 활동은 크기가 제한된 출력 큐를 사용하므로 훅은 진단 메시지를 출력하거나 stdout을 기다리지 않습니다. 종료 시 텔레메트리와 통합 호출 태스크를 조인한 뒤 Spirit을 언로드합니다. 운영자가 승인한 출력 전용 예외에 따라 작성 스레드는 250 ms 동안 큐를 비운 다음, stdout이나 stderr가 계속 막혀 있으면 해당 스레드만 분리할 수 있습니다. 이 스레드는 스케줄러, Spirit, 감사 핸들을 보유하지 않으며 상속된 파일 디스크립터 플래그를 변경하지 않습니다. `WriterStats::shutdown_abandoned`는 분리 시점에 완료가 확인되지 않은 레코드 수이며, 진행 중인 쓰기는 나중에 완료될 수 있습니다. `detached`는 이 예외의 발생 여부를 기록합니다. 이후 활동과 선택적인 최종 터미널 알림은 종료를 막는 대신 포기합니다.

모든 훅은 kernel과 상호작용하는 단일 표면인 `&mut Ctx`를 받습니다. 재정의하지 않은 훅은 no-op입니다.

`[lifecycle].enabled_hooks` manifest 필드는 최적화이자 안전 선언입니다: kernel에 발화할 훅을 알려줍니다. 생략하거나 비워 두면 모든 훅이 활성화됩니다. 특정 훅을 나열하면 kernel이 나열되지 않은 훅의 디스패치를 건너뛰어 오버헤드를 줄이고 Spirit의 공격 표면을 좁힙니다. `on_swap_out`, `snapshot`, `migrate`를 포함한 14개 훅 이름을 모두 허용하므로, 훅을 명시적으로 나열한 Spirit도 핫스왑할 수 있습니다.
