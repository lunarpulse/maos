---
title: Scheduled Invocations
sidebar_position: 8
description: Using on_schedule and manifest schedule entries for periodic Spirit tasks.
---

# Scheduled Invocations

## Problem

Your Spirit needs to run periodic tasks — a daily digest, an hourly health check, or a cron-style data pipeline. You want the kernel to fire your hook on a schedule without relying on external cron or timer infrastructure.

## Solution

Declare schedule entries in the manifest:

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

Enable the `on_schedule` hook in the lifecycle section:

```toml
[lifecycle]
enabled_hooks = ["on_load", "on_schedule", "on_idle", "on_unload"]
```

Implement the `on_schedule` handler:

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

Scheduled invocations (Story 6.4 / FR26 / ADR-025) let the kernel fire `on_schedule` at declared cadences. Each `[[schedule]]` entry in the manifest carries:

| Field | Required | Description |
|---|---|---|
| `id` | Yes | Unique identifier; delivered in `SchedulePayload.id` |
| `cadence` | Yes | Cron expression (standard 5-field) |
| `rate_limit_per_hour` | No | Maximum fires per hour (default 60) |
| `compliance_claim_ref_hex` | No | Hex-encoded compliance claim reference |
| `side_effect_scopes` | No | Declared side-effect scopes for auditability |
| `payload_b64` | No | Base64-encoded static payload delivered with each fire |

**When to use schedules vs. `on_idle`:**

- Use `[[schedule]]` when you need **calendar-aligned** timing (daily at 9 AM, every 15 minutes).
- Use `on_idle` when you need **gap-filling** work that runs whenever the Spirit has no pending frames.
- The two can coexist — a Spirit can have both scheduled invocations and idle-time processing.

**Rate limiting:** `rate_limit_per_hour` is a safety net. If the cron expression would fire more often than the limit, the kernel silently drops excess invocations. This prevents runaway schedules from consuming the Spirit's budget.

Schedule entry ids must be unique within a manifest. Duplicate ids cause a manifest parse-time rejection.
