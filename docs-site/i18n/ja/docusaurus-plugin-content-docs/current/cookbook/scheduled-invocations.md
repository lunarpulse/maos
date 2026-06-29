---
title: 스케줄된 호출
sidebar_position: 8
description: on_schedule과 manifest schedule 항목으로 주기적 Spirit 작업 수행.
review_status: machine
---

# 스케줄된 호출

## Problem

Spirit이 주기적 작업을 실행해야 합니다 — 일일 요약, 시간별 헬스 체크, 또는 cron 스타일 데이터 파이프라인. 외부 cron이나 타이머 인프라에 의존하지 않고 kernel이 스케줄에 따라 훅을 발화하기를 원합니다.

## Solution

manifest에 스케줄 항목을 선언합니다:

```toml
[[schedule]]
id = "daily-digest"
cadence = "0 9 * * *"            # cron expression: 9 AM daily
rate_limit_per_hour = 2

[[schedule]]
id = "health-check"
cadence = "*/15 * * * *"         # every 15 minutes
rate_limit_per_hour = 8
```

라이프사이클 섹션에서 `on_schedule` 훅을 활성화합니다:

```toml
[lifecycle]
enabled_hooks = ["on_load", "on_schedule", "on_idle", "on_unload"]
```

`on_schedule` 핸들러를 구현합니다:

```rust
use maos_spirit_abi::lifecycle::{Spirit, SchedulePayload};
use maos_spirit_abi::ctx::Ctx;

pub struct DigestSpirit;

impl Spirit for DigestSpirit {
    fn on_schedule<'a>(&self, ctx: &mut Ctx, payload: &SchedulePayload<'a>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }

        // Dispatch on the schedule id from the manifest entry.
        match payload.id {
            b"daily-digest" => {
                self.run_daily_digest(ctx);
            }
            b"health-check" => {
                self.run_health_check(ctx);
            }
            _ => {
                // Unknown schedule id — log and ignore.
            }
        }
    }
}

impl DigestSpirit {
    fn run_daily_digest(&self, ctx: &mut Ctx) {
        // Collect events from the last 24 hours, summarise, emit frame.
    }

    fn run_health_check(&self, ctx: &mut Ctx) {
        // Ping dependencies, report status via IAC frame.
    }
}
```

## Discussion

스케줄된 호출(Story 6.4 / FR26 / ADR-025)은 kernel이 선언된 케이던스로 `on_schedule`을 발화하게 합니다. manifest의 각 `[[schedule]]` 항목은 다음을 가집니다:

| 필드 | 필수 | 설명 |
|---|---|---|
| `id` | Yes | 고유 식별자; `SchedulePayload.id`로 전달 |
| `cadence` | Yes | cron 표현식(표준 5-필드) |
| `rate_limit_per_hour` | No | 시간당 최대 발화 수(기본 60) |
| `compliance_claim_ref_hex` | No | 16진수 인코딩 컴플라이언스 클레임 참조 |
| `side_effect_scopes` | No | 감사 가능성을 위해 선언된 부작용 스코프 |
| `payload_b64` | No | 각 발화 시 전달되는 Base64 인코딩 정적 페이로드 |

**스케줄 vs. `on_idle` 사용 시점:**

- **캘린더 정렬** 타이밍이 필요할 때(매일 오전 9시, 15분마다) `[[schedule]]`을 사용하세요.
- Spirit에 보류 중인 프레임이 없을 때마다 실행되는 **빈 공간 채우기** 작업이 필요할 때 `on_idle`을 사용하세요.
- 둘은 공존할 수 있습니다 — Spirit이 스케줄된 호출과 유휴 시간 처리를 모두 가질 수 있습니다.

**레이트 리미팅:** `rate_limit_per_hour`는 안전망입니다. cron 표현식이 한계보다 자주 발화하면 kernel이 초과 호출을 조용히 드롭합니다. 이는 통제 불능 스케줄이 Spirit의 예산을 소진하는 것을 막습니다.

스케줄 항목 id는 manifest 내에서 고유해야 합니다. 중복 id는 manifest 파스 타임 거부를 일으킵니다.
