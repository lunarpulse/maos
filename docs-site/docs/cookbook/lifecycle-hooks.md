---
title: Lifecycle Hooks
sidebar_position: 4
description: Implementing multiple lifecycle hooks in a single MAOS Spirit.
---

# Lifecycle Hooks

## Problem

Your Spirit needs to react to more than one lifecycle event — initialise state on load, process inbound frames, handle idle periods, and clean up on unload. You need to know the full hook set and the order in which the kernel fires them.

## Solution

Implement the hooks you need on the `Spirit` trait. Every hook has a default no-op body, so you only write the ones you care about:

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload, SchedulePayload};
use maos_spirit_abi::ctx::Ctx;

pub struct AnalyticsSpirit {
    event_count: std::cell::Cell<u64>,
}

impl AnalyticsSpirit {
    pub fn new() -> Self {
        Self { event_count: std::cell::Cell::new(0) }
    }
}

impl Spirit for AnalyticsSpirit {
    /// Called once when the Spirit is admitted and loaded into memory.
    fn on_load(&self, ctx: &mut Ctx) {
        // Initialise connections, load config, prepare state.
    }

    /// Called when the Spirit receives its first Start verb.
    fn on_start(&self, ctx: &mut Ctx) {
        // Begin accepting work — open listeners, start timers.
    }

    /// Called when an IAC frame arrives at the Spirit's mailbox.
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        self.event_count.set(self.event_count.get() + 1);
        // Process the inbound frame payload.
    }

    /// Called when no frames arrive for >= idle_timeout_ms.
    fn on_idle(&self, ctx: &mut Ctx) {
        // Flush buffered analytics, run compaction, emit summaries.
    }

    /// Called when the kernel pauses the Spirit (resource pressure).
    fn on_pause(&self, _ctx: &mut Ctx) {
        // Suspend background work, release optional resources.
    }

    /// Called when the kernel resumes a paused Spirit.
    fn on_resume(&self, _ctx: &mut Ctx) {
        // Re-acquire resources, restart background loops.
    }

    /// Called when the Spirit receives an Unload verb — clean up.
    fn on_unload(&self, ctx: &mut Ctx) {
        // Flush pending writes, close connections, release resources.
    }
}
```

Declare which hooks you use in the manifest so the kernel skips the rest:

```toml
[lifecycle]
enabled_hooks = [
  "on_load", "on_start", "on_frame",
  "on_idle", "on_pause", "on_resume", "on_unload",
]
```

## Discussion

The MAOS Spirit ABI defines 14 lifecycle hooks. The kernel fires them in a well-defined order:

| Phase | Hooks | When |
|---|---|---|
| Admission | `on_load` | Spirit loaded into memory |
| Running | `on_start` → `on_frame` / `on_idle` / `on_schedule` / `on_telemetry_event` / `on_consolidate` | Normal operation |
| Suspension | `on_pause` / `on_resume` | Kernel-initiated pause/resume |
| Hot-swap | `on_swap_out` / `snapshot` / `on_swap_in` / `migrate` | Version replacement |
| Teardown | `on_unload` | Spirit removal |

Every hook receives `&mut Ctx` — the single surface for interacting with the kernel. Hooks you do not override are no-ops.

The `[lifecycle].enabled_hooks` manifest field is an optimisation and a safety declaration: it tells the kernel which hooks to fire. If you leave it empty, all hooks are enabled. If you list specific hooks, the kernel skips dispatch for unlisted ones, reducing overhead and narrowing the Spirit's attack surface.
