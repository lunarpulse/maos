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

    /// Called after the kernel has released this Spirit. State is already
    /// flipped, the receipt written, capabilities revoked, and the SCB removed.
    /// KernelCtx capability and heartbeat calls fail (ScbNotFound or denied);
    /// clean up only resources that do not require live tokens.
    fn on_unload(&self, _ctx: &mut Ctx) {
        // Flush pending writes, close connections, release resources.
    }
}
```

The kernel owns release: `on_unload` is a post-release cleanup hook, not a
teardown transaction. Do not depend on a live SCB, capability tokens, or
heartbeats while it runs. Story 16-6 review 2026-09-20.

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

In a serving `maos run` root (not `--once`), the host dispatches two hooks:

- `on_telemetry_event`: the hook pump delivers every `scalar.tap` write in that root, including markers such as `halt.context_provided`, in each emitter's write order to every admitted compiled-in Spirit whose live class is `observer` while it is `Running`. An event's `seq` is a store-write identity label, unique within one root — not a clock or an ordering key.
- `on_consolidate`: every admitted compiled-in Spirit receives an empty payload ("curate now") once per its own `[scheduling].idle_window_ms` while `Running` and receiving no inbound frames.

Spawned forms (WASM components and CLI Workers) receive neither hook. Both hooks respect `enabled_hooks` and quarantine.

The telemetry subscription right comes from the operator admitting the compiled-in Observer class, not a manifest field. Every grant/revoke is audited as a `governance:telemetry-subscription:granted` / `governance:telemetry-subscription:revoked` row at the subscriber's pid, visible in `maos audit query --spirit <id>`.

Lag is counted and reported as `maos: telemetry pump lagged: <n> event(s) skipped`. A subscriber that panics three times in a row or exceeds its hook budget once is quarantined for that runtime object; a hot-swapped successor starts clean.

The eligibility check and dispatch are not atomic: a pause or unload racing a dispatch remains the unfenced FR51(a) gap (E16-A6).

Each dispatch uses the runtime snapshot whose eligibility was checked; a concurrent hot-swap cannot redirect that queued call to its successor or charge the successor's failure history. `HookPump::flush()` captures a finite queue boundary rather than waiting for all publication to stop. Cancellation accounts unread broadcast positions in `shutdown_skipped`.

Observer activity uses a bounded output queue: hooks never write diagnostics or wait for stdout. Shutdown joins telemetry and consolidation callers before unloading Spirits. The operator-approved output-only exception gives the writer 250 ms to drain, then permits that thread alone to detach if stdout or stderr remains blocked. It holds no scheduler, Spirit or audit handles and never changes inherited descriptor flags. `WriterStats::shutdown_abandoned` counts records unconfirmed at detach (an in-flight write can still finish); `detached` records the exception. Further activity and the optional final terminal notice are abandoned rather than blocking shutdown.

Every hook receives `&mut Ctx` — the single surface for interacting with the kernel. Hooks you do not override are no-ops.

The `[lifecycle].enabled_hooks` manifest field is an optimisation and a safety declaration: it tells the kernel which hooks to fire. If you omit it or leave it empty, all hooks are enabled. If you list specific hooks, the kernel skips dispatch for unlisted ones, reducing overhead and narrowing the Spirit's attack surface. All 14 hook names are accepted, including `on_swap_out`, `snapshot`, and `migrate`, so a Spirit that lists its hooks explicitly can be hot-swapped.
