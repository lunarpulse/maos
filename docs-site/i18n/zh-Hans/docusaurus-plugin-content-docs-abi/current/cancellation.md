---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `cancellation` Module {#abi-cancellation-module}

## Related {#abi-cancellation-related}

- [ADR-002](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-002-spirit-form-at-v01-subprocess-only-inproc-gated-on-measurement.md) — 취소가 런타임 무관인 이유
- [lifecycle Module](./lifecycle) — `CancellationSignal`을 폴링하는 훅
- [ctx Module](./ctx) — `Ctx::cancellation()`


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*

취소 신호 트레이트 — Spirit 훅 취소를 위한 no_std 추상화.

ADR-002는 측정에 따라 rust-inproc을 게이팅하면서 v0.1 기본값으로 subprocess 폼 Spirit을 확정합니다. 서브프로세스 Spirit은 kernel의 Tokio 런타임과 다른 프로세스에 있으며; `tokio_util::sync::CancellationToken` 인스턴스를 직접 참조할 수 없습니다. 이 트레이트는 와이어 프로토콜이 신호로 전달할 수 있는 추상화를 제공합니다.

SDK 측(std 인식, tokio 인식)은 프로덕션 구현으로 `TokioCancellationSignal`을 제공합니다; 테스트는 `NeverCancel`을 사용합니다.


## Structs {#cancellation-structs}

### `CancellationFuture` {#maos-spirit-abi-cancellation-cancellationfuture}

[`CancellationSignal::cancelled`]가 반환하는 퓨처.

**제약:** 기본 구현은 waker를 등록하지 않고 `is_cancelled()`를 폴링합니다. waker 알림에만 의존하는 실행기(Tokio 포함)는 첫 `Pending` 결과 후 재폴링하지 않아, 신호가 이미 취소되지 않은 한 이 퓨처가 무기한 멈춥니다. 훅 구현에서는 동기 폴링을 위해 `is_cancelled()`를 사용하세요; 비동기 `cancelled()` 메서드는 이를 효율적인 런타임 인식 대기로 재정의하는 프로덕션 어댑터(`TokioCancellationSignal`)를 위한 자리표시자입니다.


```rust
pub struct CancellationFuture<'a> { /* private fields */ }
```

### `NeverCancel` {#maos-spirit-abi-cancellation-nevercancel}

참조 구현: 절대 발화하지 않는 취소 신호.

트레이트 객체 디스패치 스모크 테스트와 비동기 런타임이 필요 없는 SDK 측 단위 테스트에 유용합니다.

# Example {#maos-spirit-abi-cancellation-nevercancel-example}

```rust
use maos_spirit_abi::cancellation::{CancellationSignal, NeverCancel};

let signal = NeverCancel;
assert!(!signal.is_cancelled());

// NeverCancel is the default signal used by Ctx::mock().
```


```rust
pub struct NeverCancel;
```


## Traits {#cancellation-traits}

### `CancellationSignal` {#maos-spirit-abi-cancellation-cancellationsignal}

취소 신호용 트레이트 — 훅 구현이 특정 런타임에 결합하지 않고 취소를 확인하거나 대기할 수 있게 하는 kernel 측 다리.

Story 1b.6의 D9 패턴과 구조적으로 병행:
하나의 no_std 와이어 포맷 추상화, 하나의 std 운영 어댑터.

# Example {#maos-spirit-abi-cancellation-cancellationsignal-example}

```ignore
fn on_frame(&self, ctx: &mut Ctx) {
    if ctx.cancellation().is_cancelled() {
        return; // bail early
    }
    // … do work …
}
```


```rust
pub trait CancellationSignal {
    fn is_cancelled(&self) -> bool;
}
```
