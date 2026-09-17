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

라이브 추론은 기본적으로 OS 키링을 통해 `MAOS_ANTHROPIC_API_KEY`와
`MAOS_OPENAI_API_KEY`를 확인합니다. 제한 시간 내에 키링을 사용할 수 없거나
지정된 항목이 없으면, MAOS는 composition root에서 한 번 캡처한 환경변수 값으로
대체하고 키 내용 없이 `secret.source.fallback` 감사 이벤트를 기록합니다.
헤드리스 CI에서는 `MAOS_SECRETS_BACKEND=env`를, 봉인된 파일 저장소에서는
`MAOS_SECRETS_BACKEND=encrypted-file`과 `MAOS_KMS_MASTER_KEY`를 설정하십시오.
키는 프로바이더 생성 시 확인됩니다. 요청 직전 materialization과 제한된 메모리
수명은 아직 구현되지 않았습니다.

키링이 정상이고 지정된 항목이 없으면, MAOS는 부팅 시 캡처한 환경변수 credential을
키링으로 승격하고 이후 부팅은 키링에서 확인합니다. 승격은 키 내용 없이 저널에
기록됩니다. 이후의 `maos purge`는 이 승격으로 생성된 항목을 포함해 MAOS의
네임스페이스 아래 키링 항목을 삭제합니다.

키링은 저장 중인 시크릿을 더 안전하게 보호하고 프로세스 환경 목록에서
제외합니다. 같은 호스트에서 동일한 사용자로 실행되는 다른 프로세스로부터 키를
보호하지는 않습니다.

### MAOS 상태 제거

`maos purge`는 `--yes`가 없으면 dry run입니다. 제거 전에 MAOS 소유 root와
credential을 각각 표시하고, 운영자가 만든 서명 및 설정 파일은 남기며, 실행 중인
daemon이나 offline durable operation이 store를 점유하면 거부하고, 삭제되는 root
밖에 JSON receipt를 기록합니다. purge 성공 후 Cargo 소유 binary는
`cargo uninstall maos`로 제거하십시오.

`maos purge --keep-log --yes`는 Transparency Log를 checkpoint한 뒤 보존합니다.
같은 데이터베이스를 공유할 때 shared-memory row와 principal namespace index도
보존되며, capability-token row와 기존 `shell.turn` 텍스트도 포함됩니다. dry run은
삭제 전에 보존되는 범주와 실제 개수를 표시합니다.
tenant 모드에서는 확인된 팀의 로그만이 아니라 `teams/` 하위 트리 전체가 보존되며,
dry run과 receipt는 각 형제 팀 로그를 row 개수와 함께 표시합니다. 이 보존은 FR2의
전체 제거 보장에 대한 운영자가 선택한 문서화된 예외이며, 삭제가 기본 동작입니다.

## 운영

kernel composition root는 `crates/maos-bin`에 있습니다. 운영 표면 — 투명성 로그, capability 중재, sandbox 티어, ComplianceClaim 승인, 레지스트리, yank 전파 — 는 참조 가이드에 문서화되어 있습니다:

- [배포 토폴로지](/deploy/) — 에어갭, 백업/복원, 릴리스 서명
- [문제 해결](/troubleshoot/) — 원인과 해결책을 포함한 모든 오류 코드
- [ABI 안정성](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — v1.0 ABI 호환성 매트릭스

## 에어갭 설치

네트워크 격리 환경에서는 [에어갭 배포 런북](/deploy/air-gap-deployment)과
`maosctl install --source` 오프라인 가져오기 경로를 참고하세요.
