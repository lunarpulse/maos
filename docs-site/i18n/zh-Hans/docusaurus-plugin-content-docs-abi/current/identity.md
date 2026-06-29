---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `identity` Module {#abi-identity-module}

## Related {#abi-identity-related}

- [ADR-010](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-010-address-typing.md) — `SpiritId` / `HostId` newtype 근거
- [architecture §7.1](https://github.com/lunarpulse/maos/blob/main/_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#71-same-host-the-mailbox) — IAC 프레임 종류 분류


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*

Spirit + Host 신원 타입 + FrameKind 판별자 — v0.1-β 이후 와이어 안정.

이들은 Spirit별 메일박스 라우팅(Story 3.1)과
크로스 Host A2A 주소 지정(Epic 6 Story 6.3)을 위한 v0.3-β 신원 프리미티브입니다.


## Enums {#identity-enums}

### `FrameKind` {#maos-spirit-abi-identity-framekind}

프레임 종류 판별자 — Story 1b.1 이후 와이어 안정.

아키텍처 §7.1에 따른 IAC 프레임 종류 분류의 정준 진실 원천. 배리언트 0..=6은 IAC 버스 프레임 종류입니다; 배리언트
7/8/9는 IAC 라우터를 통과하지 않는 kernel 내부 감사 종류입니다.

# Example {#maos-spirit-abi-identity-framekind-example}

```rust
use maos_spirit_abi::identity::FrameKind;

let kind = FrameKind::from_u8(0);
assert_eq!(kind, Some(FrameKind::TaskAssign));

let unknown = FrameKind::from_u8(99);
assert_eq!(unknown, None);
```


```rust
pub enum FrameKind {
    TaskAssign,
    TaskComplete,
    DecisionDispatch,
    EpistemicHalt,
    TelemetryEvent,
    ConsentRequest,
    Retract,
    CapabilityInvocation,
    SandboxBlock,
    InferenceCall,
    CliSubprocessOutput,
    ConsentRupture,
    RateLimited,
    GatewayInbound,
    GatewayOutbound,
}
```

### Inherent Items {#maos-spirit-abi-identity-framekind-inherent-items}

이 타입에 직접 구현된 메서드와 연관 함수.

### `from_u8` {#from-u8}

`u8` 판별자를 `FrameKind`로 파스하며, 알 수 없는 값은 `None`을 반환합니다.

# Example {#from-u8-example}

```rust
use maos_spirit_abi::identity::FrameKind;

assert_eq!(FrameKind::from_u8(0), Some(FrameKind::TaskAssign));
assert_eq!(FrameKind::from_u8(25), Some(FrameKind::GatewayOutbound));
assert_eq!(FrameKind::from_u8(99), None);
```


```rust
pub fn from_u8(v: u8) -> Option<Self>
```

### `SpiritRole` {#maos-spirit-abi-identity-spiritrole}

역할 기반 IAC 주소 지정을 위한 역할.

아키텍처 §7.1 주석: 프레임 종류별 채널 클래스 라우터가 발신자가
특정 Spirit이 아닌 역할을 대상으로 할 때 `SpiritRole`에서
Spirit 신원을 해결합니다. v0.3-β는 여기에 나열된 네 가지 Director 표면 역할을
지원합니다; 전체 역할 온톨로지는 Story 6.1에 제공됩니다.

# Example {#maos-spirit-abi-identity-spiritrole-example}

```rust
use maos_spirit_abi::identity::SpiritRole;

let role = SpiritRole::Worker;

match role {
    SpiritRole::Director => { /* orchestration logic */ }
    SpiritRole::Observer => { /* monitoring logic */ }
    SpiritRole::Worker => { /* task execution logic */ }
    SpiritRole::Orchestrator => { /* multi-Spirit coordination */ }
}
```


```rust
pub enum SpiritRole {
    Director,
    Observer,
    Worker,
    Orchestrator,
}
```
