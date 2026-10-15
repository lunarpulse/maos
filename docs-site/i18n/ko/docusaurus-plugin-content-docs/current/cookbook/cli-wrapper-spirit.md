---
title: CLI 래퍼 Spirit
sidebar_position: 12
description: MAOS에서 지원되는 agent CLI를 격리된 Worker로 실행합니다.
review_status: machine
---

# CLI 래퍼 Spirit

## Problem

MAOS가 호스트 네트워크나 홈 디렉터리에 접근하지 않고 지원되는 agent CLI를 Worker로 실행하게 할 수 있습니다. `[cli_wrapper]` manifest는 command, adapter posture, 요청 egress를 설명하며 T3가 Worker를 격리합니다.

## Solution

`[class]` 대신 하나의 `[cli_wrapper]` 테이블을 선언합니다. 이 예시는 Codex와 API endpoint를 선택합니다.

```toml
# A Worker manifest: [cli_wrapper] replaces [class]; never declare both.
[cli_wrapper]
command = "codex"
argv_prefix = ["exec", "--json", "--sandbox", "workspace-write"]
output_shape_version = "1.0.0"
recovery_policy = "escalate"
egress = ["api.openai.com:443"]

[cli_wrapper.posture]
stdio_shape = "ndjson_over_stdio"
control_channel = "signals"
shutdown_signal = "SIGTERM"

[sandbox]
tier = "T3"

[author]
name = "OpenAI"

[on_crash]
action = "nack"
```

운영자 grant는 manifest의 `command`와 `[author]` `name`으로 매칭되며 tier와 egress를 허용합니다. 다음과 같은 grant를 `MAOS_HOST_GRANTS`가 지정하는 TOML 파일에 저장합니다. Worker manifest에는 grant를 선언하지 않습니다.

```
[[grant]]
attested_image = "codex"
signing_key_id = "OpenAI"
permitted_tier = "T3"
permitted_egress_destinations = ["api.openai.com"]
```

운영자는 `MAOS_T3_IMAGE_LOCK_PATH`와 `MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX`도 설정하고 잠금된 image가 로컬에 있는지 확인해야 합니다. 실제 CLI에는 비어 있지 않은 `MAOS_LIVE_AGENT`가 필요합니다. Codex egress의 vendor key는 `CODEX_API_KEY`이며 daemon만 proxy용으로 읽고 Worker에는 전달하지 않습니다. `maos run <manifest.toml> [--once]`로 Worker를 시작합니다. operator door는 Worker manifest를 거부합니다.

## Discussion

**Manifest 형태:** `[cli_wrapper]`는 단일 테이블이며 `[class]`와 상호 배타적입니다. 둘 다 있으면 manifest gate 3에서 거부됩니다. `[cli_wrapper]`와 `[cli_wrapper.posture]`의 알 수 없는 key는 거부됩니다. operator door는 gate 3b에서 모든 Worker manifest를 거부합니다. `maos run` 또는 `maos run`으로 실행하는 topology file을 사용합니다.

**지원 command와 adapter 규칙:** 지원 command는 `codex`, `claude`, `worker-cli-fixture`, `worker-probe-fixture`뿐입니다. 그 외 command는 probe나 spawn 전에 거부됩니다. `codex`는 `argv_prefix`에 `exec`, `--json`, `--sandbox workspace-write`가 필요합니다. `claude`는 `--print`, `--output-format json`, `--bare`, 그리고 `sandbox.enabled = true`인 `--settings` JSON이 필요합니다. 우회 flag는 거부됩니다. 실제 `codex`와 `claude`에는 비어 있지 않은 `MAOS_LIVE_AGENT`도 필요합니다. admission probe는 지정된 binary를 host에서 환경을 비운 채 실행합니다. 실제 CLI에서는 liveness probe를 사용하고 adapter가 완료 시 output shape를 검증합니다.

**Posture 및 task 흐름:** T3에서는 `control_channel = "signals"`와 함께 `ndjson_over_stdio` 또는 `raw`만 허용됩니다. stdin을 유지하는 `json_rpc_over_stdio`, `named_pipe`, `stdin_commands`는 거부됩니다. 컨테이너에는 stdin이 없습니다. task는 마지막 argv argument로 전달되고 stdout은 Transparency Log에 bridge되며 adapter가 completion verdict를 결정합니다.

**Egress:** manifest destination은 엄격한 `host:port` 형식입니다. grant의 bare host는 `:443`을 뜻합니다. 요청된 manifest egress는 host grant destination의 부분집합이어야 합니다. upstream은 검증된 TLS를 사용하는 HTTPS이며, 명시적으로 grant된 IPv4 loopback literal만 plain HTTP를 사용합니다. Worker의 proxy 접근은 spawn별 scoped bearer로 허가되며 vendor key는 daemon만 읽습니다. 모든 proxy refusal은 `403` 전에 kind-8 row(`sandbox.block.egress.<host>:<port>`)로 journal됩니다. `maos audit query --spirit <name>`으로 확인할 수 있습니다.

**Recovery와 종료:** `recovery_policy`는 load 시 검사됩니다. `respawn_with_context`는 거부되며 생략 시 기본값입니다. 선택한 정책과 관계없이 `maos run`은 현재 아무것도 respawn하지 않습니다.

| `recovery_policy` | load 시 | Worker 종료 후 |
|---|---|---|
| `respawn_with_context` (기본값) | 거부 | `maos run`이 respawn하지 않음 |
| `respawn_fresh` | 허용 | `maos run`이 respawn하지 않음 |
| `escalate` | 허용 | `maos run`이 respawn하지 않음 |

T3 시작은 runtime이 소유합니다. MAOS는 `create` → `init`을 실행하고 runtime이 기록한 pidfile을 읽은 뒤 `start --attach`를 감독합니다. 컨테이너는 자동 제거하지 않으며 (`--rm`을 사용하지 않음) `--restart=no`를 사용합니다. 소유한 CID를 유지한 상태에서 terminal facts를 한 번 읽은 후, 해당 CID를 대상으로 bounded `rm --force --volumes`를 수행하고 `container exists`로 부재를 확인합니다. 짧은 task는 startup 대기 없이 정상 완료됩니다. Linux ≥6.15에서 payload pidfd `PIDFD_INFO_EXIT`는 의도적인 `exit(137)`, 외부 SIGKILL (`fault.signaled`), OOM (`fault.oom_killed`)을 구분합니다. 권위 있는 payload facts를 사용할 수 없으면 명시된 runtime-evidence fallback 행을 적용합니다. 정리를 확인할 수 없으면 exit 행에 typed container residue를 기록합니다. Worker unload는 post-unload registry stop을 수행하며, termination이 확인된 뒤에만 lease가 종료됩니다.

**리소스 상한:** manifest와 operator의 CPU 백분율, memory MiB, PID/태스크 수, 파일 디스크립터 제한은 각각 독립적으로 더 엄격한 값으로 결정됩니다. 생략된 Worker 기본값은 `--cpus=1`, `--memory=256m`, `--pids-limit=64`이며, 생략이 manifest 상한을 추가하지는 않습니다. `cpu_max_pct = 10`은 `--cpus=0.10`, `100`은 `--cpus=1.00`으로 매핑됩니다. `pids_max`는 별도로 `--pids-limit`을 제어하고, `fd_max`는 PID 별칭이 아닌 명시적 동일 soft/hard `--ulimit=nofile` 제한을 추가합니다. 요청된 FD 상한이 상속된 hard ceiling보다 높으면 clamp하지 않고 launch를 거부합니다. T3 경로는 Linux x64 전용입니다. 지원되는 native runtime 증명은 Ubuntu 24.04/26.04 x64 및 arm image를 포함하지만 T3는 x64만 지원합니다. macOS/Windows의 동등한 리소스 동작은 주장하지 않습니다.

Native Linux T2에서 명시적 CPU 또는 PID 제한을 사용하려면 쓰기 가능한 위임 cgroup v2 domain이 필요하며, 사용할 수 없으면 guest code 실행 전에 거부합니다. Memory-only 요청은 대신 `RLIMIT_AS`(및 선택적 `RLIMIT_NOFILE`)를 유지하면서 성공한 fallback launch마다 `ResourceCeilingDowngraded=20` 정보 lifecycle 행 하나를 기록할 수 있습니다. FD-only 및 리소스 없는 launch는 이 행을 기록하지 않습니다. 이는 state transition이 아닙니다. Native journal adapter는 live Worker 소유권/observer 등록 후, startup publication 전에 기록합니다. append 실패는 계속 panic을 일으키고 `sync_data` 오류는 무시되므로 power-loss durability 보장은 추가되지 않습니다. Native cgroup smoke는 다음 named-unit helper로 실행합니다: `python3 scripts/run_cgroup_ceiling_17_2.py run --mode delegated|undelegated --unit maos-17-2-NAME --timeout SECONDS -- COMMAND` (동일 UID rootless canary에는 `--rootless-canary`를 추가).

Root shutdown은 컨테이너 생성 전에 custody를 확보하고, 진행 중인 초기화가 끝난 뒤 제거하며, 취소된 launch가 이후에 시작되지 않도록 합니다. Exit 관측, 명시적 stop, bridge finalization, guard cleanup은 처음 정한 절대 teardown deadline을 공유합니다. Runtime 진단이 잘렸거나 runtime 자손이 pipe를 열어 둔 경우에는 `rm`이 성공해도 volume cleanup을 확인한 것으로 처리하지 않고 typed residue를 유지합니다. CID kill 실패는 이후 제거 성공과 별도로 보고합니다. `XDG_RUNTIME_DIR`을 사용할 수 없으면 임시 custody 디렉터리를 effective UID별로 구분하므로 한 사용자가 다른 사용자의 launch를 막지 않습니다. OOM 판정에는 runtime의 OOM marker도 필요합니다. pidfd evidence만으로는 signal을 증명할 수 있지만 원인까지 증명할 수는 없습니다.

**격리되지 않는 부분:** admission probe는 host에서 실행됩니다. First-party `rust-inproc` Spirits는 T0에서 daemon 안에 실행됩니다. T3에는 MAOS 전용 seccomp profile이 없습니다. MAOS는 vendor-CLI image나 entry shim을 제공하지 않으므로 이 profile에서 live Codex 또는 Claude 실행을 주장하지 않습니다.
