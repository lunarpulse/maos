---
title: 배포 토폴로지
sidebar_position: 1
description: MAOS 배포 토폴로지 — 단일 호스트, 멀티 호스트 A2A, 에어갭, contained Workers (T3).
review_status: machine
---

# 배포 토폴로지

MAOS는 네 가지 배포 토폴로지를 지원합니다. 운영 요구 사항에 맞는 것을 선택하세요.

## Single-host

가장 단순한 토폴로지: 단일 머신에서 모든 Spirit을 실행하는 하나의 MAOS kernel 프로세스입니다.

```
┌─────────────────────────────────┐
│          MAOS Kernel            │
│  ┌─────────┐  ┌─────────┐      │
│  │ Spirit A │  │ Spirit B │     │
│  └─────────┘  └─────────┘      │
│  ┌──────────────────────┐      │
│  │  Transparency Log    │      │
│  │  (SQLite)            │      │
│  └──────────────────────┘      │
└─────────────────────────────────┘
```

**사용 시점:** 개발, 테스트, 낮은 처리량의 프로덕션 워크로드.

**설정:**

```bash
# Build with default features (includes networking)
cargo build -p maos-bin --release

# Initialize and run
./target/release/maos init
./target/release/maos run
```

모든 Spirit은 인프로세스 채널로 통신합니다. Transparency Log는 `$MAOS_HOME/audit/transparency.sqlite`의 로컬 SQLite 데이터베이스입니다.

## 멀티 호스트 (A2A)

여러 MAOS kernel 인스턴스가 mTLS를 통한 Agent-to-Agent(A2A) 프로토콜로 통신합니다. 각 호스트는 자체 kernel과 자체 Spirit을 실행합니다.

```
┌──────────────────┐    mTLS/A2A    ┌──────────────────┐
│   Host A         │◄──────────────►│   Host B         │
│   MAOS Kernel    │                │   MAOS Kernel    │
│   ┌──────────┐   │                │   ┌──────────┐   │
│   │ Spirit 1 │   │                │   │ Spirit 3 │   │
│   │ Spirit 2 │   │                │   │ Spirit 4 │   │
│   └──────────┘   │                │   └──────────┘   │
└──────────────────┘                └──────────────────┘
```

**사용 시점:** 분산 워크로드, 크로스 팀 Spirit 격리, 지리적 분산.

**요구 사항:**

- 각 호스트의 mTLS 인증서(상호 인증)
- A2A 포트에서 호스트 간 네트워크 연결
- 각 호스트는 자체 Transparency Log를 유지

**구성:**

```bash
# On each host, configure A2A peer addresses
export MAOS_A2A_PEERS="host-b.example.com:9090,host-c.example.com:9090"
export MAOS_A2A_CERT="/etc/maos/tls/host.crt"
export MAOS_A2A_KEY="/etc/maos/tls/host.key"
export MAOS_A2A_CA="/etc/maos/tls/ca.crt"

./target/release/maos run
```

## 에어갭 (Air-gapped)

MAOS 바이너리가 빌드 타임에 **모든 네트워크 표면을 컴파일에서 제거**한 강화 토폴로지. 바이너리에 HTTP 클라이언트, TCP 리스너, DNS 리졸버가 존재하지 않습니다.

```
┌─────────────────────────────────┐
│      Air-Gapped Host            │
│      (no network interfaces)    │
│                                 │
│   MAOS Kernel (--features       │
│              air-gap)           │
│   ┌──────────┐                  │
│   │ Spirit A │ (offline import) │
│   └──────────┘                  │
│   ┌──────────────────────┐     │
│   │  Transparency Log    │     │
│   └──────────────────────┘     │
└─────────────────────────────────┘
```

**사용 시점:** 분류 환경, 규제 기반 네트워크 격리, defense-in-depth 배포.

**핵심 제약:**

- Spirit은 `maosctl install --source ./bundle.tar.gz`로 오프라인 임포트해야 합니다
- 원격 추론 불가 — LLM 프로바이더에 도달할 수 없습니다
- A2A 크로스 호스트 통신 불가
- HTTP를 통한 MCP 불가(stdio 전송만)
- `Scope::NetworkOutbound`와 `Scope::RegistryPoll`에 대한 역량 토큰은 발급되지 않습니다

빌드 지침과 검증은 전체 [에어갭 배포](./air-gap-deployment) 가이드를 참조하세요.

## Contained Workers (T3)

한 호스트에서 MAOS 데몬이 kernel과 first-party `rust-inproc` Spirits를 프로세스 내부에서 실행합니다(`T0`, capability tokens만 사용). Linux에서는 WASM-component Spirits가 `T2` runner 프로세스에서 실행됩니다. 각 `[cli_wrapper]` Worker는 자체 `T3` rootless container에서 실행되며, `--network=none`에서 `/proxy/egress.sock`의 데몬 egress proxy를 통해서만 네트워크에 연결합니다. 데몬은 하나의 로컬 SQLite Transparency Log에 기록합니다.

```
┌─────────────────────────────────────────────────────┐
│ Host                                                │
│  ┌───────────────────────────────────────────────┐  │
│  │ MAOS daemon                                   │  │
│  │ kernel + rust-inproc Spirits (T0)             │  │
│  │ egress proxy · Transparency Log (SQLite)      │  │
│  └──────┬────────────────────────────────────────┘  │
│         │ /proxy/egress.sock (one per Worker)       │
│  ┌──────▼─────────────────┐  ┌───────────────────┐  │
│  │ T3 rootless container  │  │ T2 runner process │  │
│  │ [cli_wrapper] Worker   │  │ WASM component    │  │
│  │ --network=none         │  │ Spirit (Linux)    │  │
│  └────────────────────────┘  └───────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**사용 시점:** 자격 증명, 홈 디렉터리 또는 네트워크에 대한 신뢰를 주지 않는 타사 에이전트 CLI를 실행할 때.

**격리 계층:**

- `rust-inproc` Spirits는 데몬 프로세스(`T0`)에서 실행되고 capability tokens로 중재되며, 프로세스 격리는 없습니다.
- Linux에서 WASM-component Spirits는 seccomp allow-list, Landlock, 위임된 cgroup v2 subtree가 적용된 `T2` runner 프로세스에서 실행됩니다.
- 각 Worker는 Linux 전용 `T3` rootless container에서 실행됩니다. 컨테이너는 `--network=none`, 읽기 전용 파일시스템을 사용하고 호스트 홈을 마운트하지 않으며, `/proxy/egress.sock`의 데몬 proxy socket을 통해서만 egress합니다.

**설계상 격리되지 않는 항목:**

- `rust-inproc` Spirits는 데몬 프로세스에서 실행됩니다.
- 승인 probe는 호스트에서 실행됩니다.
- T3 Worker manifest 리소스 제한은 CPU 백분율, memory MiB, PID/태스크 수, 파일 디스크립터별로 적용되며, 생략된 값은 runtime 기본값을 유지합니다. Native T2 cgroup/fallback 동작과 named-unit 로컬 smoke helper는 [CLI-wrapper Spirit cookbook](/cookbook/cli-wrapper-spirit)에 설명되어 있습니다.
- T3에는 MAOS 전용 seccomp 또는 AppArmor 프로필이 없습니다.
- T3는 Linux 전용이며, 다른 플랫폼에서는 실행이 거부됩니다.

**설정:** 운영자 소유 host grant와 서명된 image lock을 구성하고, rootless podman으로 lock의 digest를 사용해 이미지를 가져온 다음 Worker를 실행합니다. manifest와 grant 예제는 [CLI-wrapper Spirit cookbook](/cookbook/cli-wrapper-spirit)에 있습니다. CI 검증은 Ubuntu 24.04 및 26.04 x64 image의 rootless podman을 사용하며(T3는 x64 전용), native T2 runtime 증명은 arm image도 포함합니다.

```bash
export MAOS_T3_IMAGE_LOCK_PATH="/path/to/operator-signed-image-lock.toml"
export MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX="ED25519_PUBLIC_KEY_HEX"
export MAOS_HOST_GRANTS="/path/to/host-grants.toml"

# Replace with the image reference and digest from the signed image lock.
podman pull "IMAGE@sha256:DIGEST"
./target/release/maos run worker-manifest.toml --once
```

## 토폴로지 비교

| 기능 | 단일 호스트 | 멀티 호스트(A2A) | 에어갭 | Contained Workers (T3) |
|---------|-------------|------------------|------------|------------------------|
| 네트워크 필요 | 선택 | Yes (mTLS) | No | 선택; Worker egress는 proxy allowlist를 통해서만 가능 |
| Spirit 통신 | 인프로세스 | mTLS를 통한 A2A | 인프로세스 전용 | Spirit은 인프로세스, Worker는 stdio bridge |
| 원격 추론 | Yes | Yes | No | Worker는 proxy allowlist를 통해서만 vendor API에 연결 |
| Transparency Log | 로컬 SQLite | 호스트별 SQLite | 로컬 SQLite | 데몬이 기록하는 로컬 SQLite |
| 격리 수준 | 역량 토큰 | 역량 토큰 + 네트워크 | 역량 토큰 + 네트워크 없음 | capability tokens; Worker용 T3 container(네트워크 없음, proxy-only egress); WASM Spirit용 T2 runner |
| 복잡도 | 낮음 | 중간 | 중간 | 높음 |

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
