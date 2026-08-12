---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `ctx` Module {#abi-ctx-module}

## Related {#abi-ctx-related}

- [lifecycle Module](./lifecycle) — `Ctx`를 받는 훅
- [cancellation Module](./cancellation) — `Ctx::cancellation()`이 반환하는 `CancellationSignal`
- [deprecation Module](./deprecation) — `Ctx::deprecation_warnings()`가 반환하는 `DeprecationWarning`


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

Spirit 작성자용 컨텍스트 타입.

`Ctx`는 Spirit 훅이 호출 시점에 받는 취소 신호, 역량 핸들,
메일박스 핸들을 전달합니다. 훅이 kernel과 상호작용할 수 있는
유일한 표면입니다 — I1에 따라 Spirit은 Capability Registry를 우회할 수 없습니다.


## Structs {#ctx-structs}

### `Ctx` {#maos-spirit-abi-ctx-ctx}

모든 훅에 전달되는 Spirit 작성자용 컨텍스트.

취소 신호(장기 실행 훅이 조기 종료 가능),
불투명 역량 핸들(훅이 SDK를 통해 역량 API 사용 가능),
불투명 메일박스 핸들(SDK를 통해 IAC 프레임 송수신)을 전달합니다.

취소 신호는 `'static` 바운드로 저장되는데,
kernel이 기저 신호(예: `Arc<AtomicBool>`)를 소유하고
모든 Spirit 훅 호출보다 오래 살도록 보장하기 때문입니다. 이는
vtable 디스패치 함수에 라이프타임 매개변수가 생기는 것을 피합니다.


```rust
pub struct Ctx { /* private fields */ }
```

### Inherent Items {#maos-spirit-abi-ctx-ctx-inherent-items}

이 타입에 직접 구현된 메서드와 연관 함수.

### `cancellation` {#cancellation}

취소 신호를 빌립니다.

# Example {#cancellation-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let mut ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert!(!ctx.cancellation().is_cancelled());
```


```rust
pub fn cancellation(&self) -> &dyn CancellationSignal
```

### `capability` {#capability}

불투명 역량 핸들 — kernel이 중재 시점에 해결합니다.

# Example {#capability-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert_eq!(ctx.capability(), CapabilityHandle(100));
```


```rust
pub fn capability(&self) -> CapabilityHandle
```

### `mailbox` {#mailbox}

불투명 메일박스 핸들 — kernel이 IAC 디스패치 시점에 해결합니다.

# Example {#mailbox-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert_eq!(ctx.mailbox(), MailboxHandle(200));
```


```rust
pub fn mailbox(&self) -> MailboxHandle
```

### `deprecation_warnings` {#deprecation-warnings}

Story 7.1 v0.5 바인딩 — 현재 훅 발화 동안 Spirit 코드가 사용한
지원 중단된 ABI 표면을 관찰합니다. v0.5 ABI에 지원 중단이 없으므로
빈 슬라이스를 반환합니다.

# Example {#deprecation-warnings-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert!(ctx.deprecation_warnings().is_empty());
```


```rust
pub fn deprecation_warnings(&self) -> &[DeprecationWarning]
```

### `for_rust_inproc_hook` {#for-rust-inproc-hook}

rust-inproc 훅 디스패치를 위한 kernel 내부 `Ctx`를 생성합니다.

Story 5.1 마감: kernel 측 `HookDispatcher`는
각 훅 발화에 전달할 `Ctx`가 필요합니다. v0.3-β에서 rust-inproc 폼은
`&'static NeverCancel`을 사용하는데, kernel이 `Ctx`가 아닌
`KernelCtx`의 SCB 상태를 통해 취소를 중재하기 때문입니다.
실제 핸들은 0 값입니다(rust-inproc 폼은 이를 사용하지 않습니다;
Story 5.5x의 서브프로세스 폼이 와이어
디코드 핸드셰이크에서 이를 채웁니다).

이 생성자는 `mock` 기능 뒤에 게이팅되지 않습니다 — 이것이
rust-inproc 디스패치를 위한 프로덕션 지원 kernel 측 표면이며,
`maos-kernel-core` 내에서만 호출 가능합니다
(Spirit 작성자는 이를 호출할 수 없습니다; Spirit은 kernel로부터
완전히 구성된 `Ctx`를 받으며 스스로 구성하지 않습니다).

# Example {#for-rust-inproc-hook-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert_eq!(ctx.capability(), CapabilityHandle(100));
assert_eq!(ctx.mailbox(), MailboxHandle(200));
```


```rust
pub fn for_rust_inproc_hook(capability_handle: CapabilityHandle, mailbox_handle: MailboxHandle) -> Self
```
