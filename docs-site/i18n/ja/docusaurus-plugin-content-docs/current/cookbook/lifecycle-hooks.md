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

    /// Called when the Spirit receives an Unload verb — clean up.
    fn on_unload(&self, ctx: &mut Ctx) {
        // Flush pending writes, close connections, release resources.
    }
}
```

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

모든 훅은 kernel과 상호작용하는 단일 표면인 `&mut Ctx`를 받습니다. 재정의하지 않은 훅은 no-op입니다.

`[lifecycle].enabled_hooks` manifest 필드는 최적화이자 안전 선언입니다: kernel에 발화할 훅을 알려줍니다. 비워 두면 모든 훅이 활성화됩니다. 특정 훅을 나열하면 kernel이 나열되지 않은 훅의 디스패치를 건너뛰어 오버헤드를 줄이고 Spirit의 공격 표면을 좁힙니다.
