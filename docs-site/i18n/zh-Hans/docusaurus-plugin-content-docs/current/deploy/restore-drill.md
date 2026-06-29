---
title: 백업 및 복원 드릴
sidebar_position: 3
description: 분기별 Transparency Log 백업, 콜드 복원, Merkle 교차 검증 절차.
review_status: machine
high_risk: true
---

# 백업 및 복원 드릴 (DR-1)

Transparency Log 백업, 콜드 복원, Merkle 무결성을 검증하는 분기별 드릴입니다.

> 정식 런북: `docs/runbooks/dr-1-restore-drill.md`

**빈도:** 분기별
**담당:** 온콜 SRE / 플랫폼 운영자
**범위:** 단일 리전 Transparency Log 백업 + 콜드 복원 + Merkle 교차 검증

## 사전 요구 사항

- `maosctl` 바이너리가 PATH에 있음
- 활성 TL 데이터베이스(기본 경로: `$MAOS_HOME/audit/transparency.sqlite`)
- 백업 및 복원 대상용 쓰기 가능 스크래치 디렉터리
- RTO 측정용 스톱워치 / `time` 명령

## 절차

### 1. 백업 생성

```bash
maosctl backup create --dest /tmp/dr-drill/tl-backup.sqlite
```

SQLite 온라인 백업 API를 사용해 라이브 TL의 WAL-체크포인트 일관 스냅샷을 생성합니다. kernel이 쓰는 중에도 안전하게 실행할 수 있습니다.

### 2. 크래시 시뮬레이션

```bash
# Record the pre-crash latest timestamp for RPO verification
maosctl backup verify --backup /tmp/dr-drill/tl-backup.sqlite

# In a real drill, stop the kernel process
kill -9 $(pidof maos)
```

### 3. 백업에서 복원

**여기서 RTO 타이머를 시작합니다.**

```bash
time maosctl backup restore \
  --backup /tmp/dr-drill/tl-backup.sqlite \
  --target /tmp/dr-drill/restored/transparency.sqlite
```

### 4. Merkle 무결성 검증 (R-DR1)

복원 명령은 자동으로 Merkle 교차 검증을 실행합니다. 독립적으로 검증하려면:

```bash
maosctl backup verify --backup /tmp/dr-drill/restored/transparency.sqlite
```

이것은 복원된 데이터베이스의 모든 `frame_id` 값에서 Merkle 루트를 재계산하고 소스 루트와 바이트 단위로 비교합니다. 불일치는 백업 또는 복원 중 손상을 나타냅니다.

### 5. 쿼리 테스트 — 첫 성공적 읽기

```bash
MAOS_AUDIT_DB=/tmp/dr-drill/restored/transparency.sqlite \
  maosctl audit query
```

**쿼리가 행을 반환하면 RTO 타이머를 정지합니다.**

### 6. 결과 기록

| 측정 항목       | 값                 |
| ------------ | --------------------- |
| 드릴 날짜   | YYYY-MM-DD            |
| 백업 크기  | N MB                  |
| 프레임 수  | N                     |
| Merkle 일치 | YES / NO              |
| 측정 RTO | Ns (목표: < 4h)     |
| 운영자     | name                  |
| 메모        |                       |

## RTO 측정 방법론

- **시작:** `maosctl backup restore` 호출 시점의 경과(wall-clock) 시간
- **정지:** >= 1 행을 반환하는 첫 성공적 `maosctl audit query`의 경과 시간
- **목표:** 프로덕션 규모 TL의 경우 < 4시간

## 정직한 리스크: R8-DR

프로덕션 규모 4시간 RTO는 **CI에서 테스트 불가**합니다: CI Transparency Log는 자명하게 작습니다(< 1000 프레임, < 1초 복원). 프로덕션 규모 데이터베이스로 분기별 수동 드릴만이 신뢰할 수 있는 RTO 측정입니다. CI는 코드 경로(백업 → 복원 → Merkle 검증 → 쿼리)를 실행해 회귀를 잡지만, 규모에서 경과 RTO를 검증할 수는 없습니다.

## 정리

```bash
rm -rf /tmp/dr-drill/
```

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
