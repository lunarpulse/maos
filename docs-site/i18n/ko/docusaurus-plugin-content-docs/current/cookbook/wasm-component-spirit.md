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

[capabilities.required.iac]
send = ["spirit:peer"]

[posture]
default = "assistive"
allowed_max = "assistive"

[output_shape]
required_fields = ["response"]

[budget]
context_window_size = 4096
time_cap_seconds = 60
```

`[capabilities.required.iac]`만 선언하세요. bus 전송에는 `iac.send` 권한이 필요하며, kernel은 전달되는 각 frame을 capability token으로 중재합니다(Spirit에게 주소가 지정된 frame은 `"spirit:peer"`, 수신자가 없는 frame은 `"broadcast"`). 따라서 Spirit의 전달 내역이 `maosctl audit query --spirit`에 나타납니다. WASM world는 provider capability를 import하지 않으므로 `[capabilities.required.provider]`를 추가하지 마세요. `[class]` 안에서 trust tier는 `org-internal`, `public-untrusted`, `public-vetted` 같은 하이픈 값으로 사용합니다.

## Discussion

manifest는 형식별 결과 전에 모든 섹션을 검증합니다. 게시된/default MAOS 빌드는 Hold 2가 `wasm-host` engine을 비활성화하는 동안 이 형식을 `wasm_engine_off`로 거부합니다. `--features wasm-host`로 만든 빌드는 `maos run`을 통해 격리된 T2 runner(`maos` 옆에 설치된 `maos-wasm-runner`) 안에서 component를 실행합니다. 격리된 runner가 제공하지 않는 in-process 기능(operator door, topology, hot-swap 후속 Spirit)은 `spawned_surface_unsupported`로 거부됩니다. 이는 알 수 없는 class나 형식이 아니라 의도적인 타입화된 거부입니다.

생성된 TypeScript guest는 world의 `onStart`, `handleFrame`, `onShutdown` export를 구현합니다. guest 코드에서 halt하려면 `throw { tag: "fault", val: "<reason>" }` 값을 throw하세요. host는 이 형태를 `Halt::Fault`로 매핑합니다. guest의 WASI stdout/stderr(`console.log` 포함)는 runner의 stderr로 전달되어 `spirit.diagnostic` 행으로 journal에 기록됩니다. 이는 신뢰할 수 없는 guest 텍스트로 취급하세요. 세션마다 256행 / 64 KiB로 제한되고 초과분은 `dropped_diagnostics`에 집계되며, 너무 긴 줄은 표시와 함께 잘리고 frame이 되지 않습니다.

인트리 퍼스트파티 Rust 작성에는 [Hello-World Spirit](./hello-world-spirit)을 사용하세요. 그 레시피의 `rust-inproc` 형식과 T0 tier는 이 서드파티 component 경로와 의도적으로 다릅니다.
