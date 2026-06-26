---
title: CLI 래퍼 Spirit
sidebar_position: 12
description: cli_wrapper manifest 섹션을 사용해 외부 CLI 도구를 MAOS Spirit으로 감싸기.
review_status: machine
---

# CLI 래퍼 Spirit

## Problem

MAOS 거버넌스 하에 실행하고 싶은 기존 커맨드라인 도구 — 린터, 데이터 파이프라인, 코드 포매터 — 가 있습니다. 이것을 Rust로 다시 작성거나 Spirit 트레이트를 직접 구현하고 싶지 않습니다. CLI 래퍼 메커니즘이 kernel이 전체 라이프사이클, 역량 중재, 감시와 함께 도구를 Spirit으로 관리하게 해줍니다.

## Solution

`[cli_wrapper]` 섹션을 선언하는 manifest 전용 Spirit을 만듭니다:

```toml
[class]
name = "jq-spirit"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.1.0-alpha"
forms = ["subprocess"]
trust_tier = "local"
description = "Wraps jq as a MAOS Spirit for JSON transformation."

[author]
name = "ops-team"

[sandbox]
tier = "hardened"

[resources]
max_memory_mb = 128
max_cpu_ms = 10000

[budget]
max_inference_calls = 0
time_cap_seconds = 120

[posture]
default = "supervised"
allowed_max = "supervised"

# ── CLI Wrapper section (Story 6.2 / ADR-021) ────────────
[cli_wrapper]
command = "/usr/bin/jq"
output_shape_version = 1
recovery_policy = "restart"

[cli_wrapper.posture]
default = "supervised"
allowed_max = "supervised"

[supervision]
heartbeat_interval_ms = 5000
progress_threshold_ms = 30000
silent_failure_threshold_ms = 30000

[on_crash]
action = "restart"
```

kernel이 서브프로세스 라이프사이클을 처리합니다. 인바운드 IAC 프레임은 CLI 도구의 stdin으로 파이프되고; 도구의 stdout은 캡처되어 선언된 `output_shape_version`과 대조 검증됩니다.

## Discussion

CLI 래퍼 Spirits(Story 6.2 / ADR-021)은 기존 도구와 MAOS 거버넌스 모델 사이의 다리입니다. 핵심 설계 사항:

**상호 배타성:** manifest은 `[cli_wrapper]` 또는 네이티브 Spirit 훅 중 **하나**를 선언합니다 — 둘 다 안 됩니다. kernel은 `[cli_wrapper]` 섹션을 가지면서 `enabled_hooks`를 선언하거나 Spirit 트레이트를 구현하는 manifest을 거부합니다. 이는 프로세스 라이프사이클 소유권의 모호성을 방지합니다.

**출력 형태 검증:** `output_shape_version` 필드는 CLI 도구 stdout의 예상 구조를 선언합니다. 어드미션 시점에 kernel이 프로브 호출을 실행하고 출력을 선언된 형태와 대조 검증합니다. 관측된 출력이 일치하지 않으면 어드미션이 `CliWrapperAdmissionError::EOutputShapeMismatch`로 실패합니다 — 폴백 파싱이 없습니다(ADR-021).

**복구 정책 옵션:**

| 정책 | 동작 |
|---|---|
| `restart` | 0이 아닌 종료 시 kernel이 서브프로세스 재시작 |
| `fail` | kernel이 Spirit을 실패로 표시; 운영자 개입 필요 |
| `ignore` | kernel이 실패를 로그하고 계속(선택적 도구에 사용) |

**CLI 래퍼 vs. 네이티브 Spirit 사용 시점:**

- 도구가 이미 존재하고, 잘 테스트되었으며, 깊은 kernel 통합(IAC 라우팅, 핫스왑, 역량 중재 추론)이 필요 없을 때 `[cli_wrapper]`을 사용하세요.
- 여러 라이프사이클 훅, 핫스왑 상태 전달, 또는 세밀한 역량 스코핑이 필요할 때 네이티브 Spirit을 사용하세요.
- CLI 래퍼는 `subprocess` 폼으로 실행됩니다 — kernel의 주소 공간을 공유하지 않습니다. 이는 자연스러운 격리를 제공하지만 IPC 오버헤드를 추가합니다.

**샌드박스 의미:** CLI 래퍼는 `subprocess` 폼으로 실행되므로, `[sandbox].tier`가 스폰된 프로세스에 OS 수준 샌드박싱(seccomp, landlock)을 적용합니다. 신뢰할 수 없는 도구를 감쌀 때 `hardened` tier가 권장됩니다.
