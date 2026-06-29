---
title: Spirit SDK로 테스트
sidebar_position: 11
description: SpiritTest 하네스와 LocalRunner로 kernel 없이 Spirit 훅 테스트.
review_status: machine
---

# Spirit SDK로 테스트

## Problem

전체 MAOS kernel을 띄우지 않고 Spirit의 훅 로직을 테스트하고 싶습니다. 취소 신호, 역량 핸들, 메일박스 핸들을 제공하는 모의 `Ctx`와 올바른 라이프사이클 순서로 훅을 발화하는 하네스가 필요합니다.

## Solution

`Ctx::for_test()` 생성자를 사용하고 표준 Rust 테스트를 작성합니다:

```rust
#[cfg(test)]
mod tests {
    use maos_spirit_abi::lifecycle::Spirit;
    use maos_spirit_abi::ctx::Ctx;
    use maos_spirit_abi::cancellation::NeverCancel;

    use super::MySpirit;

    #[test]
    fn on_idle_increments_counter() {
        let spirit = MySpirit::new();

        // Ctx::for_test() builds a mock context with NeverCancel,
        // a zeroed CapabilityHandle, and a zeroed MailboxHandle.
        let mut ctx = Ctx::for_test();

        spirit.on_load(&mut ctx);
        spirit.on_start(&mut ctx);

        // Fire on_idle and assert side effects.
        spirit.on_idle(&mut ctx);
        assert_eq!(spirit.idle_count(), 1);
    }

    #[test]
    fn on_frame_processes_payload() {
        use maos_spirit_abi::lifecycle::FramePayload;

        let spirit = MySpirit::new();
        let mut ctx = Ctx::for_test();
        spirit.on_load(&mut ctx);

        let payload = FramePayload {
            data: b"hello",
            _marker: core::marker::PhantomData,
        };

        spirit.on_frame(&mut ctx, &payload);
        assert_eq!(spirit.frames_processed(), 1);
    }

    #[test]
    fn snapshot_round_trips() {
        let spirit = MySpirit::new();
        let mut ctx = Ctx::for_test();
        spirit.on_load(&mut ctx);

        // Mutate some state.
        spirit.on_idle(&mut ctx);
        spirit.on_idle(&mut ctx);

        // Snapshot and verify it deserialises.
        let snap = spirit.snapshot(&mut ctx);
        assert!(!snap.is_empty(), "snapshot should contain state");

        // Verify migration from the snapshot.
        let successor = MySpirit::new();
        let migrated = successor.migrate(&mut ctx, &snap);
        assert!(migrated.is_ok(), "same-version migration should succeed");
    }

    #[test]
    fn cancellation_aborts_early() {
        use maos_spirit_abi::cancellation::CancellationSignal;

        // A custom signal that is always cancelled.
        struct AlwaysCancel;
        impl CancellationSignal for AlwaysCancel {
            fn is_cancelled(&self) -> bool { true }
        }

        let spirit = MySpirit::new();
        // Build a Ctx with the AlwaysCancel signal.
        let mut ctx = Ctx::for_test_with_cancellation(
            &AlwaysCancel as &'static dyn CancellationSignal,
        );

        spirit.on_idle(&mut ctx);
        // Spirit should have bailed early — no work done.
        assert_eq!(spirit.idle_count(), 0);
    }
}
```

통합 수준 테스트에는 전체 라이프사이클 시퀀스를 오케스트레이션하는 `LocalRunner`를 사용합니다:

```rust
#[cfg(test)]
mod integration {
    use maos_spirit_sdk::spirit_test::LocalRunner;
    use super::MySpirit;

    #[test]
    fn full_lifecycle_smoke() {
        let mut runner = LocalRunner::new(MySpirit::new());

        // LocalRunner fires hooks in kernel order:
        // on_load → on_start → (frames/idle/schedule) → on_unload
        runner.load();
        runner.start();
        runner.idle();       // fires on_idle
        runner.unload();     // fires on_unload

        assert!(runner.completed_cleanly());
    }
}
```

## Discussion

Spirit SDK는 두 가지 테스트 표면을 제공합니다:

**1. `Ctx::for_test()` — 단위 수준 모의 컨텍스트**

- 취소 신호로 `NeverCancel`을 사용합니다(절대 발화 안 함).
- 0으로 채워진 `CapabilityHandle(0)`과 `MailboxHandle(0)`을 제공합니다.
- 비동기 런타임 불필요 — 순수 동기 테스트.
- 개별 훅 로직을 격리해 테스트하는 데 유용합니다.

**2. `LocalRunner` — 통합 수준 라이프사이클 하네스**

- kernel과 동일한 순서로 훅을 발화합니다.
- 훅이 panic하지 않고 타임아웃 내에 반환하는지 검증합니다.
- 실행 중인 kernel이나 네트워크 접근이 불필요합니다.
- 전체 라이프사이클 시퀀스를 스모크 테스트하는 데 유용합니다.

**테스트 패턴:**

- 전체 라이프사이클을 테스트하기 전에 `Ctx::for_test()`로 **각 훅을 독립적으로** 테스트하세요.
- `true`를 반환하는 커스텀 `CancellationSignal` 구현을 제공해 **취소를 테스트**하세요.
- 한 인스턴스에서 `snapshot()`을, 다른 인스턴스에서 `migrate()`를 호출해 **핫스왑을 테스트**하세요.
- 잘못된 형태의 페이로드를 주고 Spirit이 우아하게 처리하는지 단언해 **에러 경로를 테스트**하세요.

`NeverCancel` 구현은 의도적으로 최소입니다 — 프로덕션이 아닌 테스트용입니다. 프로덕션에서 kernel은 `Arc<AtomicBool>`이 지원하는 `TokioCancellationSignal`을 제공합니다.
