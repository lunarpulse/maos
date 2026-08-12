---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `deprecation` Module {#abi-deprecation-module}

## Related {#abi-deprecation-related}

- [lifecycle Module](./lifecycle) — 훅이 `Ctx::deprecation_warnings()`로 경고를 관찰
- [ctx Module](./ctx) — `Ctx::deprecation_warnings()`
- [STABILITY.md](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — 지원 중단 라이프사이클 추적


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

Story 7.1 v0.5 바인딩 — 지원 중단 경고 채널 표면.

지원 중단된 ABI 표면을 사용하는 Spirit 코드는
`Ctx::deprecation_warnings()`로 관찰 가능한 태그된 경고를 받습니다. `spirit-test` SDK는
이 경고를 테스트 출력에 표시합니다; Story 7.5a의 ABI 호환성
매트릭스 게이트(NFR-Maint-3)는 v1.0에 모든 지원 중단
표면이 일치하는 `STABILITY.md` 항목을 갖도록 단언하기 위해 이를 소비합니다.

v0.5에서 ABI는 표면할 지원 중단이 ZERO입니다 — 채널은
EMPTY-PRESENT로 제공됩니다. `Ctx::mock_with_deprecation_warnings(vec![...])`
테스트 헬퍼는 v0.5에 실제 지원 중단이 없더라도 표면화가
동작함을 `spirit-test`가 검증하게 합니다.
