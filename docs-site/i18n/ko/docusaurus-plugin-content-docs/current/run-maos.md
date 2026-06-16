---
title: "MAOS 실행"
sidebar_position: 2
description: "MAOS kernel 설치 및 운영"
---

# MAOS 실행

MAOS kernel을 설치하고 운영하는 운영자를 위한 가이드입니다.

## kernel 빌드

```bash
cargo build -p maos-bin --release
```

## 데메너 시작

```bash
./target/release/maos init    # 초기 설정 ($MAOS_HOME 생성)
./target/release/maos run     # kernel 데메너 시작
```

## 운영

kernel composition root는 `crates/maos-bin`에 있습니다. 운영 서피스 — Transparency Log, 권한 중재, sandbox tier, ComplianceClaim 승인, 레지스트리, yank 전파 — 는 참조 가이드에 문서화되어 있습니다:

- [배포 토폴로지](/deploy/) — 에어갭, 백업/복원, 릴리스 서명
- [Troubleshooting](/troubleshoot/) — 모든 에러 코드와 원인 및 해결 방법
- [ABI Stability](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — v1.0 ABI 호환성 매트릭스

## 에어갭 설치

[Air-Gap Deployment Runbook](/deploy/air-gap-deployment)을 참조하세요. `maosctl install --source` 오프라인 임포트 경로를 사용할 수 있습니다.
