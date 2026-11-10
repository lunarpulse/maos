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

### 암호화 공급자

`maos init --crypto-provider ring`은 `$MAOS_HOME/config.toml`에
`[crypto] provider = "ring"`을 기록합니다. 새 홈과 이 섹션이 없는 기존 홈의
기본값은 `ring`입니다. 초기화 명령에 플래그를 다시 지정하면 암호화 선택만
변경되며 다음 부팅부터 적용됩니다. 알 수 없거나 잘못된 선택은 사용 가능한
공급자 목록과 함께 종료 코드 78로 부팅을 거부합니다. 부팅마다 공급자, 버전,
설정 또는 기본값 출처가 `crypto.provider.selected`로 기록됩니다.
`maosctl audit query --frame-kind TelemetryEvent --intent-contains crypto.provider`로 조회합니다.
`Kernel` 출처이고 pid가 `0`인 행만 신뢰해야 합니다. Spirit도 같은 intent를
기록할 수 있으며 그 행은 선택된 공급자의 증거가 아닙니다.

v1.0에는 `ring`만 포함됩니다. 이는 교체 가능한 연결점이며 FIPS 검증이 아닙니다.
FIPS 140-3 검증 모듈은 v1.5에 계획되어 있습니다. daemon은 설치된 게시자 서명과
ComplianceClaim을 선택된 공급자로 다시 검증합니다. `maosctl spirit install`과
`maosctl import`는 여전히 ring 직접 사전 검사입니다. 명시적인
`maos run <installed-manifest>`도 ring으로 게시자 서명을 검증하고
ComplianceClaim을 평가하지 않으므로 daemon 부팅 재검증 보장에 포함되지
않습니다. vetting, yank, registry publish 검사도 ring을 직접 사용합니다.
감사 산출물 서명, 게시자 도구, rustls TLS는 이 공급자 범위 밖입니다.
20-13 L2 전에는 kernel 내부 T3 입장 검증이 여전히 `ring`을 사용합니다.


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
실행 중인 creator나 member를 죽이지 않습니다. private T3 custody는 컨테이너 제거
전에 inventory에서 검증하여 보관한 CID의 부재를 새로 확인하고, 같은 private
소유 디렉터리의 device/inode를 재검사한 뒤에만 제거합니다. Podman은 제거 시
cidfile도 삭제할 수 있으므로 inventory에서 CID가 없거나 잘못된 경우에는 이후
파일이 없어져도 제거 권한으로 삼지 않습니다. process root는 비어 있고 이름의
creator PID 부재가 확인되어야 제거합니다. live/anonymous process root, 공유
state parent와 `maos.slice`, 실행 중이거나 CID를 확인할 수 없는 spawn custody는 보존합니다.
dry run의 custody 제거 표시는 부재 확인을 조건으로 하며
읽기 전용 `ps`/`inspect`를 사용합니다. inspect 실패 때만 bounded `container exists`로
재확인하여 부재가 확인된 후보만 건너뛰며, 존재하거나 부재를 확인할 수 없으면
inspect status/stderr를 포함한 오류로 거부합니다. receipt는 schema 변경 없이
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

실행 중인 daemon에서 live compiled-in Spirits는 일반 hook 전달이 quarantine된
경우에도 forget notice를 받습니다. Researcher는 캐시된 claim과 마지막 출력을
비우고, 캐시 id를 로그와 대조한 뒤 governed recall로 가져온 현재의 살아남은
로그 payload만으로 요약을 재생성합니다. root는 결과를 검증하고 다시 검사한 뒤
유한한 tail sweep을 수행합니다. 프레임 삭제의 완료 경계는 handler 반환 시점이
아니라 **durable transactional TL marker**입니다. 마커 전에 커밋된 삭제 대상
프레임의 본문은 남지 않습니다. scope/forget/hold/redaction 감사 metadata,
IAC lineage 배열, 구조화된 수치 사실은 보존 대상으로 명시합니다. 마커 뒤에
커밋된 행은 완료 후 데이터이며 삭제했다고 주장하지 않습니다. 처리 중이거나,
hold가 있거나, 귀속을 검증할 수 없는 범위는 pending/retained로 명시합니다.
소유한 malformed kind-11 본문은 boot/pid/Spirit 귀속이 검증된 경우에만
삭제할 수 있습니다. 증명에는 실제 id가 들어가며, 최종 마커가 있어야 앞서
관찰한 malformed gap을 해소합니다. 귀속이 모호한 malformed 바이트는 보존합니다.
보존된 수치 사실은 계속 인용할 수 있습니다. 부분 cost/governance redaction은
전체 본문 tombstone이 아니므로 무관한 forget 때문에 중립적인 파생 프레임까지
삭제되지 않습니다.
HTTP503 `handler_still_running`은 기존 deadline을 유지하며 진행 중인 handler를
취소하지 않습니다. 나중의 결과를 확인하거나 같은 요청을 다시 보내십시오.
pending을 삭제 또는 완료로 읽으면 안 됩니다. 재생성만 pending인 경우는 별도입니다.
프레임 삭제가 끝나도 사용할 수 없는 작성자의 재생성은 pending으로 남을 수 있습니다.
운영자의 완료 receipt가 durable하게 기록되면, 디코딩된 principal과 선택적 reason이
같은 요청은 원래 `operation_id`, 구성 요소별 개수와 프레임 마커를 보존한 저장 결과에
`replayed: true`를 붙여 반환합니다. 이는 과거 마커 범위의 재생이며 현재 저장소에 대한
새로운 삭제 보장이 아닙니다. 인덱스 조회로 마커를 검증하고 이후 TL 행을 최대 4096개
확인합니다. 새로 일치하는 본문, 새 distillate, 현재 hold가 있거나 한도에 도달하면
새 forget을 실행합니다. 보존된 제어 metadata는 새 애플리케이션 본문이 아닙니다.
전송 요청에는 새 handle이 생기며, 완료 감사 행의 `replayed_operation_id`가 원래
작업을 가리킵니다. 이 조회 때문에 기존 route deadline을 연장하지 않습니다.
재생 중에는 다른 저장소를 다시 조사하지 않습니다. 관찰 가능한 TL tail 밖에 새 데이터가
있으면 기존 명령에서 `--reason`을 달리하여 새 요청을 제출하십시오.
기록된 결과가 없는 journal 요청만 이후 forget/pump pass에서 복구합니다.
이미 기록된 Pending 결과는 자동으로 재시도하지 않으며, 요청은 다음 boot에서
자동 재개하지 않습니다.

`maosctl forget --principal <id> --reason legal-hold`는 durable hold를 기록하고
삭제나 재생성 없이 종료 코드 3을 반환합니다. 기존 hold도 삭제를 차단합니다.
다른 legal-hold principal도 함께 명시한 프레임과 그 파생 distillate는 보존하고
typed skip으로 보고합니다. Shared와 Loom의 non-principal 조정 데이터는 partition
규칙에 따라 유지됩니다. Unicode 문자와 숫자는 id의 일부이고 문장 끝 구두점은
경계입니다. 따라서 공백 없이 CJK 텍스트에 바로 붙은 id는 감지되지 않을 수 있습니다.
감지는 의미가 아닌 id 기준이므로 id 없는 복사본, 변환되거나 escape된 id,
외부 source에서 나중에 다시 수집된 데이터는 이 보장의 범위 밖입니다.

### Spirit 제거와 inventory 증명 보관

`maosctl uninstall <id>`는 해당 Spirit에 SCB가 있으면 live door를 사용합니다.
SCB 없이 거절된 설치 패키지는 서비스 중인 root를 중지하고 잠금이 있는
offline 경로를 사용하십시오. 검색은 재사용된 pid가 아니라 stable owner identity와
admission으로 검증한 lineage를 사용합니다. legacy numeric owner는 이름이 있는
coverage gap입니다. Held/Failed 결과는 설치 generation 전체를 보존하고 사용 가능한
서명된 부분 증명의 경로를 제공합니다. 명시적인 복구 오류는 운영자가 처리해야 합니다.
패키지 rollback이 이미 수행한 삭제까지 되돌리지는 않습니다.

되돌릴 수 없는 변경 전에 서명된 잠정 gap receipt를 만듭니다. 정상적인 최종화에서는
terminal inventory와 함께 보관하지만 게시 실패 후에는 앞선 부분 receipt만 남을 수
있습니다. 첫 디렉터리 항목이나 파일명 순서가 아니라 반환 JSON의 `proof_path`를
사용하십시오. 잠정 receipt는 완료를 증명하지 않습니다. 읽을 수 없는 held 값은
보존되지만, 그 hold로 별도의 unindexed private namespace까지 덮을 수는 없습니다.
그러한 namespace가 남으면 uninstall 결과는 Failed입니다.

증명 검증은 서명뿐 아니라 정확히 열여섯 category와 typed evidence의 합계를
확인합니다. 유효한 부분 inventory가 모든 substrate의 삭제를 증명하지는 않습니다.
L1에는 명시된 AC3/KS/O4, watchdog, `kernel_private_index_hold_lease` gap이
남습니다. 프레임 마커만 마커 이전 TL 범위를 증명합니다. 같은 UID의 독립 작성자가
마커 뒤에 데이터를 다시 넣는 것을 막는 barrier는 없으며, 감사 scope/lifecycle
metadata는 보존합니다.

**최종 tail 재생성의 한계.** uninstall tail은 실제 요청을
`Pending/Refused("late-frame")`으로 기록하고, durable `redistillation` 결과에
실제 개수와 원래 id를 담습니다. 다른 실행 중인 Spirit으로 요청을 전달하지 않습니다.
서명된 프레임 증명의 본문 삭제 개수는 재생성 완료를 뜻하지 않습니다.
uninstall 반복이나 일반 forget은 기록된 Pending 요청을 자동으로 다시 구동하지
않습니다. L1에는 운영자용 재시도 명령이 없습니다.
권한이 있는 API embedding은 uninstall 밖에서 명시적으로 재시도할 수 있습니다.
알려진 subject에 `memory.forget_with_reason`을 호출하여 실제
`ForgetOutcome::Erased`를 얻고, 변경하지 않은 durable 요청을 그 결과의
`redistill` 목록에 추가한 다음
`HookPump::finish_forget(tl, &outcome, Some(live_pump))`를 호출합니다.
실제 receipt/개수를 보존하고 작성자 incarnation을 검증하십시오. subject, receipt,
id를 만들어 내거나 Suspended hold를 우회하면 안 됩니다. 알 수 없는 subject나
사용할 수 없거나 모호한 incarnation은 pending으로 남습니다. 운영자용
redistill-retry 표면은 명시적으로 ownerless/open이며 그 표면을 처음 추가하는
story가 trigger owner가 됩니다. 자동 복구를 주장하지 않습니다.

독립 verifier에는 선언된 hash-pinned Ed25519 backend가 있습니다. 지원되는
Linux CPython runner의 checkout에서 격리된 환경에 설치하십시오.

```sh
python3 -m venv .venv-erasure-verify
.venv-erasure-verify/bin/python -m pip install --require-hashes -r tools/verify-erasure/requirements.txt
.venv-erasure-verify/bin/python tools/verify-erasure/verify.py <bundle> --pubkey-file <pinned-public-key>
```

CI는 두 소비 job에 같은 backend를 제공합니다. 의존성이 없으면 vector를 skip하지
않고 실패합니다. bundle은 128 MiB 제한, owner-only, no-replace이며 subject
commitment는 signer-secret입니다. identity/timing/frame-id의 상관관계는 여전히
드러납니다. 반환된 증명을 안전하게 보관하십시오. 별도의 보관 정책이 결정되기
전에는 제품 purge가 보존된 증명을 삭제할 수 있습니다.

## 운영

kernel composition root는 `crates/maos-bin`에 있습니다. 운영 표면 — 투명성 로그, capability 중재, sandbox 티어, ComplianceClaim 승인, 레지스트리, yank 전파 — 는 참조 가이드에 문서화되어 있습니다:

- [배포 토폴로지](/deploy/) — 에어갭, 백업/복원, 릴리스 서명
- [문제 해결](/troubleshoot/) — 원인과 해결책을 포함한 모든 오류 코드
- [ABI 안정성](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — v1.0 ABI 호환성 매트릭스

### 비용 귀속

`maosctl audit cost-reconcile --month YYYY-MM --format ndjson --pricing xtask/provider-pricing.toml`은
Inference Port 토큰 사용량을 task, principal, attribution source, Spirit, provider,
model별로 묶습니다. plain 출력도 가능합니다. 보고서는 관측 정보이며 청구서가
아닙니다. egress proxy를 통과하는 Worker CLI 지출은 cost frame에 포함되지 않습니다.

`session_principal`은 운영자가 부팅 시 바인딩한 세션 principal이며 인증된 최종
사용자를 뜻하지 않습니다. `write_target_proxy`는 해당 Spirit이 쓸 수 있는
쓰기 대상의 역조회입니다. 모호하거나 귀속되지 않은 값은 `host-unallocated`에
남습니다. schema 1의 `task: none`은 이전 frame을, schema 2에서는 바인딩된
scope가 없음을 뜻합니다. 20-14 L1은 읽기 기능이며 root task와 session
바인딩은 최종 kernel landing을 기다립니다. 기존 writer는 여전히 schema 1과
proxy 귀속을 기록합니다.

NFR-Cost-1의 Spirit/task/principal 차원은 토큰 지출에 적용됩니다. CPU 시간과
storage I/O는 v1.1 계획입니다(ADR-046 R6). 청구서와 전체 ≥98% 조정은 운영
검사이며 합성 테스트나 principal별 청구 정확도의 증명이 아닙니다.

## 에어갭 설치

네트워크 격리 환경에서는 [에어갭 배포 런북](/deploy/air-gap-deployment)과
`maosctl install --source` 오프라인 가져오기 경로를 참고하세요.
