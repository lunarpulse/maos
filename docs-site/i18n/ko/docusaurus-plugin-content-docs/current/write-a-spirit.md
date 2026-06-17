---
title: "Spirit 작성하기"
sidebar_position: 1
description: "30분 첫 Spirit 경로"
---

# Spirit 작성하기 — 30분 첫 Spirit 경로

이 가이드와 `cargo generate`만으로 첫 MAOS Spirit을 만드세요.

## 1. 공식 템플릿으로 스캐폴딩

```bash
cargo generate --git https://github.com/lunarpulse/maos \
  templates/spirit-rust --name my-spirit
```

Spirit 템플릿을 가져옵니다 — [`examples/example-spirit`](https://github.com/lunarpulse/maos/tree/main/examples/example-spirit)과 동일한 템플릿입니다. 라이프사이클 훅, manifest, 스모크 테스트가 포함된 컴파일 가능한 Spirit을 얻습니다.

## 2. 훅 하나 구현

`src/lib.rs`를 열고 `on_idle` (또는 다른 라이프사이클 훅)을 편집하세요. `#[spirit]` proc-macro가 구현 블록을 kernel이 호출하는 SpiritVtable로 연결합니다.

[Manifest Schema](/manifest/latest)에서 `spirit.toml`이 선언할 수 있는 모든 필드를 확인하고, [ABI Reference](/abi/v1/)에서 전체 훅 서피스를 확인하세요.

## 3. 로컬에서 실행 — kernel 필요 없음

`LocalRunner` (및 `SpiritTest` 하네스)는 kernel 의존성 없이 모의 `Ctx`에 대해 Spirit의 훅을 실행합니다:

```bash
cargo test -p my-spirit
```

## 4. "완료"의 의미

첫 Spirit는 발행된 ABI에 대해 **컴파일**되고, 스모크 테스트가 통과하며, Butler-class 회귀 시나리오에서 올바르게 작동하면 Spirit이 완료됩니다.

## 다음 단계

- [Cookbook](/cookbook/) — 일반적인 Spirit 작업을 위한 10개 이상의 실행 가능한 패턴
- [Troubleshooting](/troubleshoot/) — 모든 에러 코드와 원인 및 해결 방법
- [Deploy](/deploy/) — 에어갭, 백업/복원, 릴리스 서명
