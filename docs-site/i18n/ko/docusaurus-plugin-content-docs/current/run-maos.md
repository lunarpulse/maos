---
title: "MAOS 실행"
sidebar_position: 2
description: "MAOS kernel 설치 및 운영"
review_status: machine
---

# MAOS 실행

MAOS kernel을 설치하고 운영하는 운영자를 위한 가이드입니다.

## kernel 빌드

```bash
cargo build -p maos-bin --release
```

## daemon 시작

```bash
./target/release/maos init    # 최초 설정 ($MAOS_HOME 생성)
./target/release/maos run     # kernel daemon 시작
```

`maos init`은 operator door의 `control.json`을 생성하는 유일한 경로이기도
합니다. 루프백 엔드포인트와 bearer 토큰을 기록하며, 변경(mutating) 동사들이
이로 인증합니다. 로테이션은 `rm $MAOS_HOME/control.json && maos init`입니다.

### 평가자(evaluator) 셸

```bash
./target/release/maos shell
```

셸은 참조 Spirit(`hello-spirit`)을 스케줄러에 실제 pid로 적재한 뒤 REPL을
렌더링합니다. 지시는 `@hello-spirit <메시지>`로 입력하고 `Ctrl-D`로 종료합니다.
의도적으로 모호한 지시 — 예: `@hello-spirit refactor src/main.rs to be more
idiomatic` — 는 Spirit을 `task.acceptance_criterion.ambiguous`로 정지(halt)시킵니다:

```
[HALT task.acceptance_criterion.ambiguous] 'more idiomatic' is undefined; …
halt 01M2GJ3GJMJRBB7PWQXPKWJT3K — type a clarification, or:
  maosctl halt resolve 01M2GJ3GJMJRBB7PWQXPKWJT3K --spirit hello-spirit \
    --kind provided-context --text "…"
```

**두 표면 어디서든** 해결할 수 있습니다 — REPL에 설명(clarification)을 직접
입력하거나, 다른 터미널에서:

```bash
maosctl halt list --spirit hello-spirit          # halt_id 표시
maosctl halt resolve <halt_id> --spirit hello-spirit \
  --kind provided-context --text "idiomatic = clippy-clean, no unwrap"
```

두 표면은 하나의 메커니즘(operator door)을 공유합니다. 레지스트리가 전환되는
순간 REPL이 해결을 렌더링하고, Spirit은 당신의 설명과 함께 **진행(proceed)**합니다.
이후 `maos audit query --spirit hello-spirit`는 halt 행과 `operator.halt-resolve`
완료 행 — 완료 여부 포함 — 를 보여줍니다.

### 추론 모드 (ADR-064)

kernel의 추론은 캐셋(hermetic)에서 실행하거나, 캐셋에 기록하거나, 라이브
프로바이더를 호출할 수 있습니다:

```bash
MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=<cassette.json> maos shell
MAOS_INFERENCE_MODE=record MAOS_REPLAY_CASSETTE=<out.json> maos shell
MAOS_INFERENCE_MODE=live    maos shell
```

### 프로바이더 키 (현재)

라이브 추론은 환경변수에서 프로바이더 키를 읽습니다:
`MAOS_ANTHROPIC_API_KEY` 또는 `MAOS_OPENAI_API_KEY`. OS 키링 저장은 계획
중입니다(Story 16-4). 그 전까지는 환경변수가 유일한 공급원입니다.

## 운영

kernel composition root는 `crates/maos-bin`에 있습니다. 운영 표면 — 투명성 로그, capability 중재, sandbox 티어, ComplianceClaim 승인, 레지스트리, yank 전파 — 는 참조 가이드에 문서화되어 있습니다:

- [배포 토폴로지](/deploy/) — 에어갭, 백업/복원, 릴리스 서명
- [문제 해결](/troubleshoot/) — 원인과 해결책을 포함한 모든 오류 코드
- [ABI 안정성](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — v1.0 ABI 호환성 매트릭스

## 에어갭 설치

네트워크 격리 환경에서는 [에어갭 배포 런북](/deploy/air-gap-deployment)과
`maosctl install --source` 오프라인 가져오기 경로를 참고하세요.
