---
title: 릴리스 서명
sidebar_position: 4
description: Ed25519 릴리스 아티팩트 서명, 검증, 키 순환 절차.
review_status: machine
high_risk: true
high_risk: true
---

# 릴리스 서명

MAOS 릴리스 아티팩트는 Ed25519으로 서명됩니다. 서명 키는 감사 서명 키 및 역량 토큰 서명 키와 **구별**됩니다.

> 정식 런북: `docs/runbooks/release-signing.md`

## 키 출처

| 키 | 위치 | 형식 |
|-----|----------|--------|
| 릴리스 서명(개인) | CI 시크릿 `RELEASE_SIGNING_KEY` | 16진수 인코딩 32바이트 Ed25519 시드 |
| 릴리스 서명(공개) | `crates/maos-audit/src/release_verify.rs::RELEASE_PUBKEY` | `[u8; 32]` const |
| 바이너리에 번들 | 모든 `maos` / `maosctl` 바이너리 | 컴파일 포함 const |

## 최초 키 생성

```bash
# Generate a new release-signing key
maosctl audit keygen --output /tmp/release-signing.key

# The output file contains a hex-encoded 32-byte seed.
# Store the hex string as the CI secret RELEASE_SIGNING_KEY.
# Derive the public key and update RELEASE_PUBKEY in release_verify.rs.
```

## 서명 흐름 (자동화 — CI)

1. 릴리스 태그: `git tag v0.5.0 && git push --tags`
2. CI가 `maos-linux-amd64`, `maos-linux-arm64`, `maos-darwin-arm64`를 빌드합니다
3. CI가 각(네이티브) 바이너리에서 `check-mock-not-in-release`를 실행합니다
4. CI가 `sha256sum maos-*`로 `SHA256SUMS`를 생성합니다
5. CI가 `xtask release-verify --sign`으로 `SHA256SUMS`에 서명합니다
6. CI가 `.sig`를 첨부하여 GitHub Releases에 게시합니다
7. CI가 `xtask release-verify --verify`로 자체 검증합니다

## 검증 흐름 (운영자)

```bash
# Download release artifacts to a local directory
mkdir maos-v0.5.0 && cd maos-v0.5.0
# Download: maos-linux-amd64, SHA256SUMS, SHA256SUMS.sig

# Verify with the bundled public key (offline-capable)
maosctl install --from-local . --verify-only

# Or via xtask (CI gate)
cargo run -p xtask -- release-verify --verify \
  --sha256sums SHA256SUMS \
  --sig SHA256SUMS.sig \
  --artifacts-dir .
```

## 키 순환

1. 새 키 쌍을 생성합니다(위 "최초 키 생성" 참조)
2. `crates/maos-audit/src/release_verify.rs`의 `RELEASE_PUBKEY`를 갱신합니다
3. GitHub Settings에서 CI 시크릿 `RELEASE_SIGNING_KEY`를 갱신합니다
4. 새 릴리스에 태그를 붙입니다 — 새 키가 새 아티팩트에 서명합니다
5. 이전 아티팩트는 원래 키로 검증 가능합니다(공개키가 그 시점에 빌드된 바이너리에 번들되어 있음)

### 긴급 순환(키 노출)

1. 즉시 CI 시크릿을 폐기합니다
2. 새 키 쌍을 생성합니다
3. 영향 받은 모든 릴리스 아티팩트를 재서명 및 재게시합니다
4. `RELEASE_PUBKEY`를 갱신하고 포인트 릴리스를 출시합니다
5. 보안 자문을 게시합니다(SECURITY.md)

## 검증 알고리즘

```
signature = Ed25519(sha256(SHA256SUMS_bytes))
```

이는 `sealed_export::sign_bundle`(Story 9.1 FR44)이 사용하는 것과 동일한 `sha256(content) -> Ed25519 sign` 관용입니다. 다이제스트는 `SHA256SUMS` 파일의 원시 바이트에 대해 계산됩니다(개별 파일 해시가 아님).

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
