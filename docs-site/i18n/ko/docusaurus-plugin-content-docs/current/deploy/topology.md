---
title: 배포 토폴로지
sidebar_position: 1
description: MAOS 배포 토폴로지 — 단일 호스트, 멀티 호스트 A2A, 에어갭, 컨테이너 격리.
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

## 컨테이너 격리 (Container-isolated)

각 Spirit이 자체 컨테이너에서 실행되며 kernel이 컨테이너 런타임으로 오케스트레이션합니다. kernel의 역량 토큰 시행을 넘어 프로세스 수준 및 파일시스템 수준 격리를 제공합니다.

```
┌──────────────────────────────────────┐
│          Host                        │
│  ┌────────────────────────────────┐  │
│  │       MAOS Kernel              │  │
│  └────────────────────────────────┘  │
│  ┌──────────┐  ┌──────────┐         │
│  │Container │  │Container │         │
│  │ Spirit A │  │ Spirit B │         │
│  └──────────┘  └──────────┘         │
│  ┌────────────────────────────────┐  │
│  │  Shared Transparency Log      │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

**사용 시점:** 멀티 테넌트 프로덕션, 신뢰할 수 없는 Spirit 워크로드, 프로세스 격리를 요구하는 컴플라이언스 환경.

**격리 계층:**

1. **역량 토큰(Capability tokens)** — kernel 수준 스코프 시행(항상 활성)
2. **컨테이너 네임스페이스** — PID, 네트워크, 마운트, 사용자 네임스페이스 격리
3. **Seccomp/AppArmor** — 시스템 콜 필터링

**네트워크 네임스페이스 격리 설정:**

```bash
# Run each Spirit subprocess in its own network namespace
# (the kernel handles this when configured for container mode)
unshare --net -- ./target/release/maos run --spirit hello-spirit
```

## 토폴로지 비교

| 기능 | 단일 호스트 | 멀티 호스트(A2A) | 에어갭 | 컨테이너 격리 |
|---------|-------------|------------------|------------|--------------------|
| 네트워크 필요 | 선택 | Yes (mTLS) | No | 선택 |
| Spirit 통신 | 인프로세스 | mTLS를 통한 A2A | 인프로세스 전용 | 인프로세스 |
| 원격 추론 | Yes | Yes | No | Yes |
| Transparency Log | 로컬 SQLite | 호스트별 SQLite | 로컬 SQLite | 공유 SQLite |
| 격리 수준 | 역량 토큰 | 역량 토큰 + 네트워크 | 역량 토큰 + 네트워크 없음 | 역량 토큰 + 컨테이너 |
| 복잡도 | 낮음 | 중간 | 중간 | 높음 |

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
