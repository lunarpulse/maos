---
title: 배포
sidebar_position: 0
description: MAOS 배포 옵션 개요 — 토폴로지, 에어갭, 백업, 릴리스 서명.
review_status: machine
---

# 배포

MAOS는 단일 호스트 개발부터 에어갭(air-gap) 프로덕션 환경까지 다양한 배포 토폴로지를 지원합니다. 이 섹션은 운영 배포 가이드를 다룹니다.

## 가이드

| 가이드 | 설명 |
|-------|-------------|
| [배포 토폴로지](./topology) | 단일 호스트, 멀티 호스트(A2A), 에어갭, 컨테이너 격리 모드 |
| [에어갭 배포](./air-gap-deployment) | 컴파일 타임 네트워크 제거를 통한 네트워크 격리 배포 |
| [백업 및 복원 드릴](./restore-drill) | Transparency Log 백업, 복원, Merkle 검증 |
| [릴리스 서명](./release-signing) | Ed25519 릴리스 아티팩트 서명 및 검증 |

## 빠른 시작

첫 배포에서는 [단일 호스트 토폴로지](./topology#single-host)와 기본 기능 세트로 시작하세요. 프로덕션 배포는 [에어갭 가이드](./air-gap-deployment)와 [릴리스 서명](./release-signing) 절차를 검토해야 합니다.

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
