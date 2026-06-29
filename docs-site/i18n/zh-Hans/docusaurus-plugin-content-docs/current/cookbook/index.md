---
title: Cookbook
sidebar_position: 1
description: MAOS Spirit을 빌드, 배포, 운영하기 위한 실용적 패턴.
review_status: machine
---

# Cookbook

일반적인 MAOS 작업을 위한 소형 레시피입니다. 모든 패턴은 동일한 구조를 따릅니다: **Problem**(필요한 것), **Solution**(복사할 수 있는 동작 코드), **Discussion**(언제 왜 사용하는지).

## 시작하기

| 패턴 | 요약 |
|---|---|
| [Hello-World Spirit](./hello-world-spirit) | 단일 `on_idle` 훅을 가진 최소 Spirit — 30분 경로 |
| [Manifest 필드](./manifest-fields) | 스키마 v3의 모든 섹션을 다루는 전체 `spirit.toml` manifest |
| [라이프사이클 훅](./lifecycle-hooks) | 하나의 Spirit에서 여러 라이프사이클 훅 구현 |

## 신뢰성 및 운영

| 패턴 | 요약 |
|---|---|
| [취소 처리](./cancellation-handling) | 장기 실행 훅에서 취소 신호 확인 |
| [역량 스코핑](./capability-scoping) | 역량 토큰을 안전하게 요청하고 사용 |
| [에러 처리](./error-handling) | 타입화된 에러와 복구 클래스 |
| [핫스왑 마이그레이션](./hot-swap-migration) | `snapshot()` + `migrate()`를 통한 상태 전달 |

## 고급 패턴

| 패턴 | 요약 |
|---|---|
| [스케줄된 호출](./scheduled-invocations) | `on_schedule`과 `[[schedule]]` 항목을 통한 주기적 작업 |
| [게이트웨이 통합](./gateway-integration) | Telegram, Slack, Discord를 위한 `GatewaySubmodule` 구현 |
| [CLI 래퍼 Spirit](./cli-wrapper-spirit) | 외부 CLI 도구를 Spirit으로 감싸기 |
| [컴플라이언스 클레임](./compliance-claim) | Spirit에 `ComplianceClaimEnvelope` 부착 |
| [Spirit SDK로 테스트](./testing-with-spirit-sdk) | `SpiritTest` 하네스와 `LocalRunner` 사용 |
