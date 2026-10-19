---
title: 서드파티 WASM Component Spirit
sidebar_position: 2.5
description: T2 샌드박스 경계에서 WASM component로 서드파티 TypeScript Spirit을 빌드합니다.
review_status: machine
---

# 서드파티 WASM Component Spirit

## Problem

MAOS 프로세스에 서드파티 코드를 넣지 않고 TypeScript로 서드파티 Spirit을 작성하려고 합니다. 지원되는 형식은 TypeScript 스캐폴드에서 빌드하고 T2 샌드박스 경계로 선언하는 자체 완비 WASM component입니다.

## Solution

`templates/spirit-ts`에서 프로젝트를 스캐폴드한 뒤, 공개 npm 의존성을 설치하고 component를 빌드합니다:

```bash
cargo generate --git https://github.com/lunarpulse/maos \
  templates/spirit-ts --name my-wasm-spirit
cd my-wasm-spirit
npm install
npm run build && npm run componentize
```

스캐폴드는 `@bytecodealliance/componentize-js`를 `0.23.0`으로 직접 고정합니다. 지원되는 Node 범위는 `^22.20 || ^24.12 || >=25`입니다. `componentize-js`는 component를 `dist/spirit.wasm`에 작성합니다.

다음의 완전한 스키마 v5 manifest로 component를 선언합니다:

```toml
[class]
name = "my-wasm-spirit"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 5
min_substrate_version = "0.1.0"
forms = ["wasm-component"]
trust_tier = "public-untrusted"
description = "A third-party TypeScript WASM component Spirit."
artifact = "dist/spirit.wasm"

[author]
name = "you"

[sandbox]
tier = "T2"

[resources]
cpu_max_pct = 25
memory_max_mb = 64
fd_max = 64
pids_max = 64

[capabilities.required.iac]
send = ["spirit:peer"]

[capabilities.required.log]
recall = true
fetch = true

[posture]
default = "assistive"
allowed_max = "assistive"

[output_shape]
required_fields = ["response"]

[budget]
context_window_size = 4096
time_cap_seconds = 60
```

bus 전송에는 `[capabilities.required.iac]`를 선언하세요. kernel은 전달되는 각 frame을 `iac.send` capability token으로 중재합니다(Spirit에게 주소가 지정된 frame은 `"spirit:peer"`, 수신자가 없는 frame은 `"broadcast"`). 따라서 Spirit의 전달 내역이 `maosctl audit query --spirit`에 나타납니다. 스키마 v5의 `[capabilities.required.log]` table은 recall과 fetch 권한을 각각 부여합니다. 두 boolean의 기본값은 모두 `false`이고, 알 수 없는 key는 거부됩니다. 스키마 5 미만에서는 `[capabilities.required.iac]`와 마찬가지로 기능 저하 경고와 함께 table이 제거됩니다. WASM world는 provider capability를 import하지 않으므로 `[capabilities.required.provider]`를 추가하지 마세요. `[class]` 안에서 trust tier는 `org-internal`, `public-untrusted`, `public-vetted` 같은 하이픈 값으로 사용합니다.

## 자신이 보낸 frame 회상하기

`maos:spirit@2.1.0` world는 두 함수를 제공하는 `maos:spirit/recall@2.1.0`을 import합니다.

```wit
recall: func(filter: recall-filter) -> result<recall-page, recall-error>;
fetch: func(frame-id: list<u8>) -> result<fetch-response, recall-error>;
```

`recall`은 payload가 아닌 메타데이터를 반환합니다. 필터에는 선택적인 kind, 시간 범위, cursor, 정확히 일치하는 intent와 0~1024 범위의 `limit`을 지정할 수 있습니다. kind 문자열은 `TaskAssign`처럼 `FrameKindLabel`의 정확한 표기를 사용합니다. 반환된 불투명 `next-cursor`를 전달하면 이어서 조회할 수 있습니다. 응답 크기 제한에 맞추기 위해 페이지가 요청한 limit보다 짧을 수 있습니다. `fetch`는 16바이트 frame id를 받아 저장된 redacted payload를 자르지 않고 메타데이터 및 frame origin과 함께 반환합니다.

두 함수는 **`handle-frame` 안에서만** 사용할 수 있습니다(TypeScript에서는 `handleFrame`). `on-start`, `on-shutdown` 또는 그 밖의 위치에서 호출하면 `unavailable`을 반환합니다. recall은 runner의 기존 stdio 채널을 사용하며 디스크립터, 소켓, 시스템 호출을 추가하지 않습니다.

범위는 **현재 데몬 실행에서 자신이 보낸 항목만**입니다. Spirit은 자신이 보낸 bus frame(raw kind 0–6)과 MCP invocation(kind 18)을 회상할 수 있습니다. 수신한 frame, 다른 발신자의 frame, 감사 행, 이전 데몬 실행의 frame은 공개되지 않습니다. `recall`은 이를 제외하고, 범위 밖에 있는 기존 행을 `fetch`하면 `scope-violation`을 반환합니다. 이전 incarnation은 설계상 회상할 수 없습니다. 재시작을 넘어 정보를 유지하려면 memory tier와 distillate를 사용하세요.

수신한 consent-bearing frame은 동의 평가가 아니라 **발신자 범위 때문에 보류**됩니다. guest가 주장한 consent envelope는 recall 권한을 부여하지 않습니다. participant recall(수신자 또는 역할 수신자), consent envelope 영속화, native와 WASM Spirit 모두의 동의 평가는 `17-8-participant-recall-and-consent-envelopes`에서 담당합니다. 이 인터페이스가 해당 동작을 제공한다는 뜻은 아닙니다.

### 오류와 감사 기록

| `recall-error` case | 발생 조건 |
|---|---|
| `scope-violation` | fetch 대상이 다른 발신자나 데몬 실행의 기존 행이거나, 감사 행처럼 공개되지 않는 kind입니다. |
| `frame-not-found` | 요청한 frame id에 해당하는 행이 없습니다. |
| `invalid-request(string)` | cursor나 id가 잘못되었거나(id는 16바이트여야 함), kind label이 알려져 있지 않거나, timestamp가 `i64::MAX`를 초과합니다. 유효하지만 공개되지 않는 kind 필터는 대신 빈 페이지를 반환합니다. |
| `limit-exceeded(u32)` | 요청한 limit이 1024를 초과합니다. 오류에는 최댓값이 포함됩니다. |
| `capability-denied` | 권한 선언이 없거나, 정책·운영자 거부·할당량 소진으로 호출별 발급이 거부됩니다. recall과 fetch에는 별도 권한이 필요합니다. |
| `consent-withheld` | story 17-8의 동의 평가를 위해 예약되어 있습니다. 발신자 전용 recall은 동의를 평가하지 않습니다. |
| `reply-too-large(u64)` | 페이지 항목 하나 또는 전체 fetch payload조차 frame 바이트 제한과 codec 확장 제한을 충족하지 못합니다. 오류에는 응답 바이트 상한이 포함됩니다. payload는 절대 잘리지 않습니다. |
| `unavailable` | `handle-frame` 밖에서 호출했거나, 저장소를 사용할 수 없거나, recall 채널이 닫히거나 실패했습니다. 내부 저장소 메시지는 노출되지 않습니다. |

데몬은 승인된 호출자 identity와 boot nonce를 사용합니다. guest는 identity나 token을 제공하지 않습니다. 검증을 통과하고 허용된 호출마다 새 capability를 발급하고, 데이터를 공개하기 전에 token이 포함된 invocation 감사 행을 기록한 뒤 token을 폐기합니다. 모든 거부는 정확히 하나의 governance receipt인 `log.recall.refused:<code>` 또는 `log.fetch.refused:<code>`를 기록하며, `<code>`는 WIT 오류 case 이름입니다. 검증이나 발급 단계의 거부는 invocation 행 없이 receipt만 기록합니다. 빈 페이지를 포함한 성공 호출에는 거부 receipt가 추가되지 않습니다. 이 감사 행 자체는 회상할 수 없습니다.

in-band 프로토콜 위반은 `recall.channel.closed:protocol-violation`을 기록하고 세션을 종료합니다. 잘못된 응답을 받으면 runner는 `unavailable`을 반환하고, 이후 recall I/O를 중단하고, 현재 export를 마친 뒤 타입화된 `recall protocol desync` 오류로 종료합니다. 반면 EOF이면 export 이후 정상 종료합니다.

## Discussion

manifest는 형식별 결과 전에 모든 섹션을 검증합니다. 게시된/default MAOS 빌드는 Hold 2가 `wasm-host` engine을 비활성화하는 동안 이 형식을 `wasm_engine_off`로 거부합니다. `--features wasm-host`로 만든 빌드는 `maos run`을 통해 격리된 T2 runner(`maos` 옆에 설치된 `maos-wasm-runner`) 안에서 component를 실행합니다. 격리된 runner가 제공하지 않는 in-process 기능(operator door, topology, hot-swap 후속 Spirit)은 `spawned_surface_unsupported`로 거부됩니다. 이는 알 수 없는 class나 형식이 아니라 의도적인 타입화된 거부입니다.

생성된 TypeScript guest는 world의 `onStart`, `handleFrame`, `onShutdown` export를 구현합니다. guest 코드에서 halt하려면 `throw { tag: "fault", val: "<reason>" }` 값을 throw하세요. host는 이 형태를 `Halt::Fault`로 매핑합니다. guest의 WASI stdout/stderr(`console.log` 포함)는 runner의 stderr로 전달되어 `spirit.diagnostic` 행으로 journal에 기록됩니다. 이는 신뢰할 수 없는 guest 텍스트로 취급하세요. 세션마다 256행 / 64 KiB로 제한되고 초과분은 `dropped_diagnostics`에 집계되며, 너무 긴 줄은 표시와 함께 잘리고 frame이 되지 않습니다.

인트리 퍼스트파티 Rust 작성에는 [Hello-World Spirit](./hello-world-spirit)을 사용하세요. 그 레시피의 `rust-inproc` 형식과 T0 tier는 이 서드파티 component 경로와 의도적으로 다릅니다.
