---
title: Hello-World Spirit
sidebar_position: 2
description: 약 30분 만에 단일 on_idle 훅을 가진 최소 MAOS Spirit 빌드.
review_status: machine
---

# Hello-World Spirit

## Problem

제로에서 실행되는 Spirit까지 가장 빠른 경로를 원합니다. 게시된 ABI에 대해 컴파일되고, 하나의 훅을 구현하며, 스모크 테스트를 통과하는 프로젝트가 필요합니다 — 모두 약 30분 안에.

## Solution

공식 템플릿에서 프로젝트를 스캐폴드합니다:

```bash
cargo generate --git https://github.com/lunarpulse/maos \
  templates/spirit-rust --name hello-spirit
```

`spirit.toml`에 최소 manifest를 추가합니다:

```toml
[class]
name = "hello-spirit"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"
description = "A minimal hello-world Spirit."

[author]
name = "you"

[sandbox]
tier = "baseline"

[resources]
max_memory_mb = 64
max_cpu_ms = 1000

[budget]
max_inference_calls = 0
time_cap_seconds = 60
```

`src/lib.rs`에 Spirit 트레이트를 구현합니다:

```rust
use maos_spirit_abi::lifecycle::Spirit;
use maos_spirit_abi::ctx::Ctx;

pub struct HelloSpirit;

impl Spirit for HelloSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        // Called when no inbound frames arrive for >= idle_timeout_ms.
        // This is where a minimal Spirit does its work.
        if ctx.cancellation().is_cancelled() {
            return;
        }
        // Your logic here — log, emit a frame, update state, etc.
    }
}
```

스모크 테스트를 실행합니다:

```bash
cargo test -p hello-spirit
```

## Discussion

`on_idle`은 Spirit에 보류 중인 작업이 없을 때 자동으로 발화하므로 가장 단순한 진입점입니다. IAC 프레임 라우팅, 스케줄된 호출, 또는 어떤 외부 트리거도 설정할 필요가 없습니다 — kernel이 인바운드 프레임 없이 구성 가능한 `idle_timeout_ms` 윈도우(기본 30 000 ms)가 경과하면 `on_idle`을 발화합니다.

이 패턴이 올바른 시작점인 경우:

- MAOS Spirit 모델을 처음 배우는 경우.
- Spirit이 자체 완비적 주기적 작업(폴링, 요약, 헬스 체크)을 수행하는 경우.
- 더 많은 훅을 추가하기 전 컴파일되는 기준을 원하는 경우.

모든 훅은 취소 신호, 역량 핸들, 메일박스 핸들을 전달하는 `&mut Ctx`를 받습니다. 최소 Spirit에서도 비싼 작업 전에 `ctx.cancellation().is_cancelled()`를 확인해야 합니다 — [취소 처리](./cancellation-handling)를 참조하세요.
