---
title: "MAOS 이해"
sidebar_position: 3
description: "MAOS 아키텍처, 불변성, 신뢰 모델"
---

# MAOS 이해

MAOS 아키텍처, 헌법적 불변성, 신뢰 경계에 대한 멘탈 모델을 만드는 독자를 위한 문서입니다.

## 핵심 아이디어

MAOS는 OS가 프로세스를 호스팅하는 방식으로 **LLM 기반 Spirit을 호스팅하는 kernel**입니다: Spirit는 manifest에 필요한 것을 선언하고, kernel이 모든 권한을 중재하며, 모든 중요한 작업은 전달 전에 기록됩니다.

## 핵심 개념

| 개념 | 의미 |
|---|---|
| **Spirit** | LLM 기반 에이전트, manifest에서 로드, sandbox tier로 격리 |
| **Manifest** | Spirit의 신원, 권한, 제약 조건 선언 ([스키마 참조](/manifest/latest)) |
| **kernel** | MAOS 런타임: 권한 중재, 불변성 시행, 라이프사이클 관리 |
| **Transparency Log** | 추가 전용 감사 추적 — 모든 중요한 작업은 전달 전에 기록 |
| **ComplianceClaim** | Spirit의 신원을 ComplianceClaim 컴플라이언스 상태에 바인딩하는 암호화 봉투 |
| **Capability Token** | 시간/범위 제한, PID 바인딩된 접근 자격증명 |

## 아키텍처

- **헥사고널 설계** — kernel은 포트 트레잇 정의; 어댑터는 플러가블
- **14개의 헌법적 불변성** — CI 게이트에 의해 시행 ([`docs/invariants/`](https://github.com/lunarpulse/maos/tree/main/docs/invariants) 참조)
- **ABI Stability Triple** — `(kernel_version, abi_version, manifest_schema_version)` N-1 지원 / N-2 거부 정책

## 신뢰 모델

- 권한 중재: 모든 Spirit 작업에는 범위 제한, 시간 제한 토큰 필요
- sandbox tier: T0 (격리 없음) → T3 (컨테이너 격리)
- Transparency Log는 전달 전에 기록 — 로그에 없으면 발생하지 않은 것
- ComplianceClaim 봉투는 승인 시 검증

[`SECURITY.md`](https://github.com/lunarpulse/maos/blob/main/SECURITY.md)에서 전체 신뢰 모델을 확인하세요.

## 다음 단계

- [Manifest Schema](/manifest/latest) — Spirit이 선언할 수 있는 모든 필드
- [Migration Guides](/migrate/) — 브레이킹 체인지 마이그레이션 경로
- [ABI Reference](/abi/latest) — `maos-spirit-abi` 생성된 API 문서
