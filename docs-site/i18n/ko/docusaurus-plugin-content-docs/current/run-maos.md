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

### MCP 서버 bearer

각 라이브 MCP 서버(calendar, slack, linear, figma, web, arxiv, github,
citation-graph의 `MAOS_MCP_<SERVER>_URI`)는 같은 시크릿 저장소에서
`MAOS_MCP_<SERVER>_BEARER`(키링 항목 `mcp-<server>-bearer`)로 bearer를 가져옵니다.
bearer는 해당 서버에만 `Authorization: Bearer`로 전송되며, 저장된 값이 없으면
`Authorization` 헤더를 보내지 않습니다. 클라이언트는 먼저 MCP 2026-07-28로
요청하고, 2025-11-25 서버에는 `initialize` 핸드셰이크와 `Mcp-Session-Id`로
대체합니다. `maos purge`는 프로바이더 키와 함께 bearer 항목을 삭제합니다.

### MAOS 상태 제거

`maos purge`는 `--yes`가 없으면 dry run입니다. 제거 전에 MAOS 소유 root와
credential을 각각 표시하고, 운영자가 만든 서명 및 설정 파일은 남기며, 실행 중인
daemon이나 offline durable operation이 store를 점유하면 거부하고, 삭제되는 root
밖에 JSON receipt를 기록합니다. purge 성공 후 Cargo 소유 binary는
`cargo uninstall maos`로 제거하십시오.

Linux에서 확인된 purge는 이 home의 T3 컨테이너를 workspace보다 먼저 제거합니다.
두 MAOS identity label, 일치하는 이름, 이 home의 workspace root 아래를 가리키는
`/workspace` mount가 모두 일치해야 합니다. 다른 home 또는 소유권을 확인할 수 없는
labelled 컨테이너는 제거 명령만 표시하며 label이 없는 컨테이너는 무시합니다.
`maos.slice/spirit-*` leaf는 비어 있고 creator가 종료된 경우에만 제거하며,
실행 중인 creator나 member를 죽이지 않습니다. private T3 custody는 CID 부재가
확인된 spawn만 제거합니다. process root는 비어 있고 이름의 creator PID 부재가
확인되어야 제거합니다. live/anonymous process root, 공유 state parent와
`maos.slice`는 보존합니다. CID가 없거나 실행 중인 spawn custody도 보존합니다.
dry run의 custody 제거 표시는 부재 확인을 조건으로 하며
읽기 전용 `ps`/`inspect`만 호출합니다. receipt는 schema 변경 없이
`podman://<CID>`, `cgroup://<path>`, custody 디렉터리 행을 추가합니다.
Worker workspace residue와 마찬가지로 `left (<residue>)` 행이 있어도 status는
`complete`일 수 있으므로 모든 행을 확인하십시오. user-namespace mapping과
podman storage/configuration은 수정하지 않습니다.

일치하는 metadata는 위조 불가능한 증명이 아닙니다. 같은 rootless UID의 다른
container creator는 label, 이름, mount를 위조할 수 있으며 same-UID runtime
control은 privilege boundary가 아닙니다. bare `podman`은 운영자가 신뢰하는
PATH에서 찾습니다. MAOS 작성 residue가 남으면 기존 `next_step` 문자열과 CLI
마지막 줄에 `complete with residue: N left`를 표시하고 exit 1을 반환합니다.
receipt status 어휘와 schema는 바꾸지 않습니다. HOME은 설정된 절대 경로여야 하며,
HOME 자체/상위 purge root와 `..` 경로는 거부합니다. legacy ancestor는 symlink를
따르지 않고 검사하며 HOME이 없을 때 공유 `/tmp` fallback을 purge하지 않습니다.

`maos purge --keep-log --yes`는 Transparency Log를 checkpoint한 뒤 보존합니다.
같은 데이터베이스를 공유할 때 shared-memory row와 principal namespace index도
보존되며, capability-token row와 기존 `shell.turn` 텍스트도 포함됩니다. dry run은
삭제 전에 보존되는 범주와 실제 개수를 표시합니다.
tenant 모드에서는 확인된 팀의 로그만이 아니라 `teams/` 하위 트리 전체가 보존되며,
dry run과 receipt는 각 형제 팀 로그를 row 개수와 함께 표시합니다. 이 보존은 FR2의
전체 제거 보장에 대한 운영자가 선택한 문서화된 예외이며, 삭제가 기본 동작입니다.

### principal 잊기

`maosctl forget --principal <id>`는 principal의 Private namespace와 index를
삭제하고, 작성자와 관계없이 토큰 경계로 식별된 id가 있는 Transparency Log
프레임을 지우며, 해당 프레임에서 파생된 distillate도 redaction 처리합니다.
JSON 결과에는 지워진 프레임 id, redaction된 distillate id, 살아남은 source
reference가 있는 재증류 요청, 각 요청의 결과가 포함됩니다.

실행 중인 daemon은 일반 hook이 quarantine된 경우에도 live compiled-in Spirit에
forget notice를 전달합니다. Researcher는 캐시된 claim과 마지막 출력을 비우고,
캐시 id를 로그와 대조한 뒤 governed recall로 가져온 현재의 살아남은 로그
payload만으로 요약을 재생성합니다. root는 결과를 검증하고 다시 검사하며,
진행 중이던 pass가 쓴 프레임도 마지막으로 지운 뒤 완료합니다. 중단된 cascade의
미완료 요청은 재시도 시 복구됩니다. 전달하지 못한 purge와 재생성 오류는 typed
결과로 남으며, 이미 끝난 삭제를 재생성 오류만으로 실패라고 보고하지 않습니다.
offline 또는 부재 중인 작성자는 조용히 생략되지 않고 typed pending으로 기록됩니다.
pending은 다음 boot에서 자동 재개하지 않습니다.

`maosctl forget --principal <id> --reason legal-hold`는 durable hold를 기록하고
삭제나 재생성 없이 종료 코드 3을 반환합니다. 기존 hold도 삭제를 차단합니다.
다른 legal-hold principal도 함께 명시한 프레임과 그 파생 distillate는 보존하고
typed skip으로 보고합니다. Shared와 Loom의 non-principal 조정 데이터는 partition
규칙에 따라 유지됩니다. Unicode 문자와 숫자는 id의 일부이고 문장 끝 구두점은
경계입니다. 따라서 공백 없이 CJK 텍스트에 바로 붙은 id는 감지되지 않을 수 있습니다.
감지는 의미가 아닌 id 기준이므로 id 없는 복사본, 변환되거나 escape된 id,
외부 source에서 나중에 다시 수집된 데이터는 이 보장의 범위 밖입니다.

## 운영

kernel composition root는 `crates/maos-bin`에 있습니다. 운영 표면 — 투명성 로그, capability 중재, sandbox 티어, ComplianceClaim 승인, 레지스트리, yank 전파 — 는 참조 가이드에 문서화되어 있습니다:

- [배포 토폴로지](/deploy/) — 에어갭, 백업/복원, 릴리스 서명
- [문제 해결](/troubleshoot/) — 원인과 해결책을 포함한 모든 오류 코드
- [ABI 안정성](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — v1.0 ABI 호환성 매트릭스

## 에어갭 설치

네트워크 격리 환경에서는 [에어갭 배포 런북](/deploy/air-gap-deployment)과
`maosctl install --source` 오프라인 가져오기 경로를 참고하세요.
