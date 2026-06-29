---
title: 에어갭 배포
sidebar_position: 2
description: 컴파일 타임 네트워크 표면 제거로 네트워크 격리 환경에 MAOS 배포.
review_status: machine
high_risk: true
---

# 에어갭 배포

**에어갭 배포(air-gap deployment)**는 빌드 타임에 모든 네트워크 표면을 컴파일에서 제거한(`--features air-gap`) 상태로 MAOS Host 바이너리를 실행합니다. 결과 바이너리에는 네트워킹 코드가 전혀 없습니다 — HTTP 클라이언트, TCP 리스너, DNS 리졸버가 없습니다.

> 정식 런북: `docs/runbooks/ag-1-air-gap-deployment.md`

## 사전 요구 사항

| 항목 | 상세 |
|------|--------|
| Rust 툴체인 | stable >= 1.88 |
| 빌드 플래그 | `--no-default-features --features air-gap` |
| CI 게이트 | `xtask check-air-gap`가 산출물 바이너리에서 통과 |
| 호스트 접근 | 아웃바운드 네트워크 불필요 |

## 빌드 절차

```bash
# 1. Build the air-gap binary
cargo build -p maos-bin --release --no-default-features --features air-gap

# 2. Verify no network symbols leaked
cargo xtask check-air-gap \
  --binary target/release/maos \
  --dirty-fixture target/debug/dirty-network-fixture

# 3. (Optional) Run the netns corroborating harness (requires root/CAP_SYS_ADMIN)
sudo bash tests/air-gap-netns-corroborate.sh target/release/maos
```

## 호스트 수준 네트워크 시행

컴파일 타임 네트워크 표면 제거에도 불구하고, defense-in-depth는 호스트 수준 시행을 요구합니다.

### 옵션 A: 네트워크 네임스페이스 격리

```bash
# Run the daemon inside a network namespace with no interfaces
unshare --net -- ./target/release/maos init
```

### 옵션 B: 방화벽 규칙 (iptables / nftables)

```bash
# Block all outbound traffic from the maos user
iptables -A OUTPUT -m owner --uid-owner maos -j DROP
iptables -A INPUT  -m owner --uid-owner maos -j DROP
```

### 옵션 C: SELinux / AppArmor

`maos` 바이너리를 `network` 접근 클래스를 거부하는 프로파일로 한정합니다.

## Spirit 임포트 (오프라인)

에어갭 환경은 레지스트리에서 Spirit을 가져올 수 없습니다. 오프라인 임포트 흐름을 사용하세요:

```bash
# On a networked machine: export a signed Spirit bundle
maosctl export --spirit hello-spirit --output hello-spirit.tar.gz

# Transfer to air-gapped host via removable media

# On the air-gapped host: import and verify
maosctl install --source ./hello-spirit.tar.gz
```

임포트는 다음을 검증합니다:

- SHA-256 manifest에 대한 Ed25519 서명
- 번들 무결성(나열된 모든 파일 존재, 해시 일치)
- trust tier 하한선(운영자 정책 시행)

## 역량 토큰 안내

에어갭 모드에서:

- `Scope::NetworkOutbound` 토큰은 **절대 발급되지 않습니다**(네트워크 표면이 없음)
- `Scope::RegistryPoll` 토큰은 **절대 발급되지 않습니다**(레지스트리 클라이언트 없음)
- 로컬 전용 스코프(`Scope::MemoryRead`, `Scope::FileRead` 등)는 정상 작동합니다
- Spirit 스케줄링, 저널링, 감사는 변경 없이 계속됩니다

## Transparency Log

Transparency Log(SQLite)는 에어갭 모드에서 동일하게 작동합니다. 모든 프레임 방출, 거버넌스 이벤트, 감사 증명이 로컬에 기록됩니다. 로그는 외부 검토를 위해 `maos audit query`로 추출할 수 있습니다.

## 제약 사항

> **정직한 리스크 공개(R8-AG):** 에어갭 빌드는 컴파일 타임에 네트워크 *표면*을 제거합니다. 이것이 손상된 Spirit(또는 빌드 타임에 끌어온 악의적 의존성)이 네트워크 외 채널(예: 파일시스템, IPC, 시그널)을 통해 I/O를 시도할 수 없다고 보장하지는 않습니다. 에어갭 기능은 defense-in-depth 전략의 한 **계층**이지, 독립적인 보안 경계가 아닙니다.

1. **라이브 레지스트리 동기화 불가** — Spirit을 오프라인으로 임포트해야 합니다
2. **원격 추론 불가** — LLM 프로바이더에 도달할 수 없습니다; 추론이 필요한 Spirit은 `ProviderError::Unconfigured`를 받습니다
3. **HTTP를 통한 MCP 불가** — MCP 서버는 stdio 전송을 사용하거나 사용 불가합니다
4. **A2A 크로스 호스트 불가** — TCP/mTLS 전송이 컴파일에서 제거되었습니다; 인프로세스 루프백(컴파일된 경우)만 사용 가능합니다
5. **모바일 푸시 불가** — Halt 알림이 모바일 기기에 도달할 수 없습니다

## CI 검증

`xtask check-air-gap` 게이트(R-AG1)가 CI에서 실행되어:

1. 에어갭 바이너리를 빌드합니다
2. `nm --demangle`로 심볼 테이블을 스캔하여 네트워크 관련 심볼을 찾습니다
3. 발견되면 실패합니다
4. 실제로 `TcpStream::connect`를 링크하는 더티 픽스처가 올바르게 거부되는지 검증합니다 — 게이트가 공허하지 않음을 증명합니다

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
