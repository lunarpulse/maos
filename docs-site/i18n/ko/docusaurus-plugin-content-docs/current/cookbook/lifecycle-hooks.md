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

`[cli_wrapper]` Worker는 Spirit 형태가 아니며 두 훅 모두 받지 않습니다. WASM 컴포넌트는 `lifecycle` export를 통해 두 훅을 모두 받습니다([아래](#spawned-wasm-component-spirits)). 텔레메트리 구독은 `on_telemetry_event`를 명시적으로 나열할 때만 이루어집니다. 두 훅 모두 `enabled_hooks`와 격리 상태를 따릅니다.

텔레메트리 구독 권한은 manifest 필드가 아니라 운영자가 컴파일 내장 Observer 클래스를 어드미션하는 데서 나옵니다. 모든 부여/회수는 구독자의 pid에 `governance:telemetry-subscription:granted` / `governance:telemetry-subscription:revoked` 행으로 감사 기록되며, `maos audit query --spirit <id>`에서 볼 수 있습니다.

지연으로 건너뛴 이벤트 수는 집계되며 `maos: telemetry pump lagged: <n> event(s) skipped`로 보고됩니다. 연속 세 번 패닉하거나 훅 예산을 한 번 초과한 구독자는 해당 런타임 객체에 대해 격리됩니다. 핫스왑된 후속 객체는 격리 이력 없이 시작합니다.

적격성 검사와 디스패치는 원자적이지 않습니다. 디스패치와 경합하는 일시 정지나 언로드는 여전히 펜싱되지 않은 FR51(a) 공백(E16-A6)입니다.

각 디스패치는 적격성을 검사한 런타임 스냅샷을 사용합니다. 동시에 핫스왑이 일어나도 대기 중인 호출이 후속 객체로 바뀌거나 후속 객체의 실패 이력에 반영되지 않습니다. `HookPump::flush()`는 모든 발행이 멈추기를 기다리지 않고 유한한 큐 경계를 캡처합니다. 취소 시 읽지 않은 브로드캐스트 위치는 `shutdown_skipped`에 집계됩니다.

Observer 활동은 크기가 제한된 출력 큐를 사용하므로 훅은 진단 메시지를 출력하거나 stdout을 기다리지 않습니다. 종료 시 텔레메트리와 통합 호출 태스크를 조인한 뒤 Spirit을 언로드합니다. 운영자가 승인한 출력 전용 예외에 따라 작성 스레드는 250 ms 동안 큐를 비운 다음, stdout이나 stderr가 계속 막혀 있으면 해당 스레드만 분리할 수 있습니다. 이 스레드는 스케줄러, Spirit, 감사 핸들을 보유하지 않으며 상속된 파일 디스크립터 플래그를 변경하지 않습니다. `WriterStats::shutdown_abandoned`는 분리 시점에 완료가 확인되지 않은 레코드 수이며, 진행 중인 쓰기는 나중에 완료될 수 있습니다. `detached`는 이 예외의 발생 여부를 기록합니다. 이후 활동과 선택적인 최종 터미널 알림은 종료를 막는 대신 포기합니다.

모든 훅은 kernel과 상호작용하는 단일 표면인 `&mut Ctx`를 받습니다. 재정의하지 않은 훅은 no-op입니다.

`[lifecycle].enabled_hooks` manifest 필드는 최적화이자 안전 선언입니다: kernel에 발화할 훅을 알려줍니다. 생략하거나 비워 두면 모든 훅이 활성화됩니다. 특정 훅을 나열하면 kernel이 나열되지 않은 훅의 디스패치를 건너뛰어 오버헤드를 줄이고 Spirit의 공격 표면을 좁힙니다. `on_swap_out`, `snapshot`, `migrate`를 포함한 14개 훅 이름을 모두 허용하므로, 훅을 명시적으로 나열한 Spirit도 핫스왑할 수 있습니다.

## Spawned WASM component Spirits

`maos:spirit@2.4.0`에 맞춰 빌드한 `wasm-component` Spirit은 선택적인 `maos:spirit/lifecycle` 인터페이스(world `spirit-lifecycle`)를 export할 수 있습니다. 그러면 kernel은 컴파일 내장 Spirit과 같은 디스패처, `enabled_hooks` 게이트, 예산, 격리 규칙으로 해당 컴포넌트에 도달합니다. export가 없는 컴포넌트(2.0–2.3 컴포넌트 전부, 또는 world `spirit`로 빌드한 것)는 계속 실행되며 호출되지 않을 뿐입니다. kernel의 probe에는 "lifecycle 없음"으로 응답합니다.

| 트리거 (프로덕션 호출자) | Export |
|---|---|
| 유휴 구간 (idle watchdog) | `idle()` |
| 통합 구간 (consolidation watchdog) | `consolidate(batch)` |
| `scalar.tap` 쓰기 (훅 펌프) | `telemetry(event)` |
| `[[schedule]]` 발화 (schedule watchdog) | `schedule(payload)` |
| 일시 정지 / 재개 | `pause()` / `resume()` |
| 핫스왑 | 선행 버전에서 `snapshot()`, 이어서 후속 버전에서 `swap-in(state)` (`--policy migrator`에서는 `migrate(state)`가 먼저) |

- **한 번에 하나의 export, 프레임 턴 중에는 절대 호출하지 않음.** 대기 중인 프레임 턴이 트리거보다 먼저 처리되므로, 바쁜 트리거 호출자가 메일박스를 굶기지 못합니다.
- **예산.** 각 export는 `[budget].time_cap_seconds`와 자체 명령어 fuel 아래에서 실행됩니다. 초과한 export는 kernel의 예산 초과 행을 만들고 세션이 끝납니다(자식 프로세스 종료, `spirit.lifecycle.deadline` 기록). 다른 export 뒤에 대기하다가 예산의 절반 안에 시작하지 못한 트리거는 `maos: lifecycle trigger dropped` 줄을 남기고 버려집니다. 놓친 예산은 바쁜 세션의 것이므로 패닉이나 격리 스트라이크가 되지 않습니다.
- **실패.** 모든 export는 `result<_, lifecycle-error>`(`declined(string)` 또는 `failed(string)`)를 반환합니다. 디스패처는 거부를 패닉한 훅으로 기록하고 평소의 회계를 적용합니다. 어떤 export도 프레임을 내보내지 않습니다. kernel 트리거에는 인바운드 intent 계보가 없으므로, 효과는 import나 다음 프레임 턴을 통해 일어납니다. 훅 ABI는 게스트에 capability 토큰을 전달하지 않습니다.
- **import.** `idle`, `consolidate`, `telemetry`, `schedule`은 `recall`, `context`, `skills`를 호출할 수 있으며, export마다 턴당 recall 한도에 집계됩니다. `pause`, `resume`, `snapshot`, `swap-in`, `migrate`는 import가 불가능(`unavailable`)하므로, 스왑 단계는 스왑 시점의 로그에 의존하지 않습니다.
- **텔레메트리는 명시적 나열이 필요합니다.** 컴포넌트는 `[lifecycle].enabled_hooks`가 `on_telemetry_event`를 명시할 때만 `telemetry`를 받습니다. 빈 목록은 구독하지 않습니다(다른 모든 훅과 다릅니다). 같은 `governance:telemetry-subscription:granted` / `…:revoked` 행이 기록됩니다. 펌프는 이벤트를 구독자에게 나란히 전달하며, 각 구독자를 `[budget].time_cap_seconds`의 95%와 5초 중 작은 값까지만 기다립니다(초과는 예산 초과로 보아 격리합니다). 예산의 80~95%에 끝나는 구독자가 이벤트 세 번 연속이면 역시 격리됩니다.
- **스케줄.** `[[schedule]]` 항목은 모든 형태에 대해 어드미션 시점에 전달되므로, 컴포넌트의 스케줄은 첫 시작부터 발화합니다. Story 20-9의 kernel 랜딩(두 번째 랜딩) 전까지는, 선언된 부수 효과 권한이 거부된 예약 발화도 토큰 없이 실행됩니다.

### Hot swap, upgrade and cold swap

`maosctl spirit upgrade <id> --to <manifest>`는 `maos run`이 시작했든 루트가 설치본에서 부팅했든 컴포넌트를 제자리에서 교체합니다. 후속 버전도 컴포넌트여야 하고(`swap_form_mismatch`), 운영자가 이 pid에 어드미션한 범위를 넘는 capability를 선언할 수 없으며(`swap_capability_widened`), `[lifecycle].enabled_hooks`에서 `on_swap_in`을 켜 두어야 하고(`swap_hook_disabled`), 두 버전 모두 `lifecycle`을 export해야 합니다(`spawned_swap_unsupported`. 실행 중인 컴포넌트가 아직 시작 중이면 `spawned_swap_not_ready`이므로 다시 시도하십시오). 아티팩트는 대상 manifest 옆에서 해석되며 그 디렉터리 안에 있어야 합니다(밖으로 나가는 심볼릭 링크는 `artifact_escapes_origin`). 이 거부는 모두 무엇이든 교체되기 전에 일어나며, 실행 중인 버전은 계속 서비스합니다. 커밋된 후속 버전이 채택되기를 기다리는 동안 발화한 훅은 유실되지 않고 그 후속 버전을 위해 대기열에 들어갑니다.

후속 버전은 먼저 실행되고 probe됩니다. 이후 스왑은 턴 경계를 기다립니다. 진행 중인 턴은 자신의 lease 아래에서 선행 버전에서 끝나고, `snapshot`이 만들어지며, 후속 버전이 `swap-in`으로 받습니다. pid, 그 pid가 가진 모든 capability 토큰, 작업 메모리, 보류 중인 halt가 유지됩니다. 선행 버전의 자식 프로세스는 토큰을 회수하지 않고 은퇴합니다. 커밋된 스왑을 스왑 후 모니터가 되돌리면 선행 버전의 자식 프로세스는 이미 사라진 상태이므로, 세션은 기록되는 사유 `swap_reverted_after_retire`로 끝나고 Spirit은 언로드됩니다. 자동으로 복원되지 않습니다.

`--policy cold-swap`은 데몬이 컴포넌트를 부팅할 때 쓰는 것과 같은 어드미션 경로로 후속 버전을 새 pid에 어드미션합니다. 토큰은 언로드로 회수되고 상태는 이어지지 않습니다. 어드미션은 컴포넌트가 시작되어 lifecycle probe에 응답한 뒤에야 반환하며, 컴포넌트가 아니거나 시작에 실패한 후속 버전은 되돌려지고 거부됩니다. 후속 버전이 거부되면 보존된 manifest로 선행 버전을 또 다른 새 pid에 복원합니다(`cold_swap_reverted`). `[cli_wrapper]` 대상은 Spirit 형태가 아니며 `cli_wrapper_is_not_a_spirit_form`을 유지합니다.
