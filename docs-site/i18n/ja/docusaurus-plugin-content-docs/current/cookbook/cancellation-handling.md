---
title: 취소 처리
sidebar_position: 5
description: 장기 실행 Spirit 훅에서 취소 신호를 확인해 조기 종료.
review_status: machine
---

# 취소 처리

## Problem

Spirit 훅이 장기 실행 작업을 수행합니다 — 큰 데이터셋 반복, 여러 추론 호출, 다단계 파이프라인. kernel이 Spirit을 언로드하거나 스왑해야 하면 훅이 즉시 빠져나가야 합니다. 취소 신호를 무시하면 kernel의 감시 워치독이 `progress_threshold_ms` 후 강제 킬로 에스컬레이션합니다.

## Solution

훅의 자연스러운 양보 지점에서 `ctx.cancellation().is_cancelled()`를 폴링합니다:

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload};
use maos_spirit_abi::ctx::Ctx;

pub struct BatchProcessor;

impl Spirit for BatchProcessor {
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        let items = parse_work_items(payload);

        for (i, item) in items.iter().enumerate() {
            // Check cancellation before each unit of work.
            if ctx.cancellation().is_cancelled() {
                // Optionally persist progress so a successor can resume.
                save_checkpoint(i);
                return;
            }
            process_item(item);
        }
    }

    fn on_idle(&self, ctx: &mut Ctx) {
        // Even short hooks should check cancellation at the top.
        if ctx.cancellation().is_cancelled() {
            return;
        }
        run_compaction();
    }
}

// Stubs for illustration.
fn parse_work_items(_payload: &FramePayload) -> Vec<WorkItem> { vec![] }
fn process_item(_item: &WorkItem) {}
fn save_checkpoint(_index: usize) {}
fn run_compaction() {}
struct WorkItem;
```

## Discussion

`CancellationSignal` 트레이트는 Spirit 폼에 걸친 취소를 추상화합니다:

- **인프로세스 Spirit**은 kernel의 Tokio 런타임이 지원하는 신호(내부적으로 `Arc<AtomicBool>`)를 받습니다.
- **서브프로세스 Spirit**은 와이어 프로토콜을 통해 메시지로 신호를 받습니다.

이 추상화 덕분에 폼과 무관하게 코드가 동일하게 작동합니다. 트레이트는 두 표면을 노출합니다:

1. **`is_cancelled() -> bool`** — 동기 폴링; 루프 본문과 훅 상단에서 사용합니다.
2. **`cancelled() -> impl Future`** — 비동기 대기; 프로덕션 어댑터(`TokioCancellationSignal`)에 유용하지만 기본 구현은 waker를 등록하지 않으므로 훅 코드에서는 `is_cancelled()`를 선호하세요.

**확인 시점:**

- 모든 훅의 상단, 어떤 작업 전에.
- 루프 내부, 매 반복 전(타이트 루프의 경우 N번마다).
- 어떤 I/O 또는 추론 호출 전후.

**무시하면 생기는 일:**

kernel의 감시 워치독이 훅 진행을 추적합니다. 취소 후 `progress_threshold_ms`(기본 30 000 ms) 내에 훅이 반환되지 않으면 kernel이 에스컬레이션합니다 — 먼저 강제 태스크 중단, 그다음 전체 Spirit 언로드. 프로덕션 Spirit에서 취소 확인은 선택이 아닙니다.

테스트에서 SDK는 항상 `false`를 반환하는 참조 구현인 `NeverCancel`을 제공합니다 — 그래서 단위 테스트는 비동기 런타임 없이도 훅 로직을 실행할 수 있습니다.
