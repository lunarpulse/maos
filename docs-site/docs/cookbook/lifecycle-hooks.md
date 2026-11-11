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

A `[cli_wrapper]` Worker is not a Spirit form and receives neither hook. A WASM component receives both through its `lifecycle` export ([below](#spawned-wasm-component-spirits)); it is subscribed to telemetry only by listing `on_telemetry_event` explicitly. Both hooks respect `enabled_hooks` and quarantine.

The telemetry subscription right comes from the operator admitting the compiled-in Observer class, not a manifest field. Every grant/revoke is audited as a `governance:telemetry-subscription:granted` / `governance:telemetry-subscription:revoked` row at the subscriber's pid, visible in `maos audit query --spirit <id>`.

Lag is counted and reported as `maos: telemetry pump lagged: <n> event(s) skipped`. A subscriber that panics three times in a row or exceeds its hook budget once is quarantined for that runtime object; a hot-swapped successor starts clean.

The eligibility check and dispatch are not atomic: a pause or unload racing a dispatch remains the unfenced FR51(a) gap (E16-A6).

Each dispatch uses the runtime snapshot whose eligibility was checked; a concurrent hot-swap cannot redirect that queued call to its successor or charge the successor's failure history. `HookPump::flush()` captures a finite queue boundary rather than waiting for all publication to stop. Cancellation accounts unread broadcast positions in `shutdown_skipped`.

Observer activity uses a bounded output queue: hooks never write diagnostics or wait for stdout. Shutdown joins telemetry and consolidation callers before unloading Spirits. The operator-approved output-only exception gives the writer 250 ms to drain, then permits that thread alone to detach if stdout or stderr remains blocked. It holds no scheduler, Spirit or audit handles and never changes inherited descriptor flags. `WriterStats::shutdown_abandoned` counts records unconfirmed at detach (an in-flight write can still finish); `detached` records the exception. Further activity and the optional final terminal notice are abandoned rather than blocking shutdown.

Every hook receives `&mut Ctx` — the single surface for interacting with the kernel. Hooks you do not override are no-ops.

The `[lifecycle].enabled_hooks` manifest field is an optimisation and a safety declaration: it tells the kernel which hooks to fire. If you omit it or leave it empty, all hooks are enabled. If you list specific hooks, the kernel skips dispatch for unlisted ones, reducing overhead and narrowing the Spirit's attack surface. All 14 hook names are accepted, including `on_swap_out`, `snapshot`, and `migrate`, so a Spirit that lists its hooks explicitly can be hot-swapped.

## Spawned WASM component Spirits

A `wasm-component` Spirit built against `maos:spirit@2.4.0` can export the optional `maos:spirit/lifecycle` interface (world `spirit-lifecycle`). The kernel then reaches it through the same dispatcher, `enabled_hooks` gate, budgets and quarantine as a compiled-in Spirit. A component without the export (any 2.0–2.3 component, or one built against world `spirit`) keeps running and is simply never called; it answers the kernel's probe as "no lifecycle".

| Trigger (production caller) | Export |
|---|---|
| Idle window (idle watchdog) | `idle()` |
| Consolidation window (consolidation watchdog) | `consolidate(batch)` |
| `scalar.tap` write (hook pump) | `telemetry(event)` |
| `[[schedule]]` fire (schedule watchdog) | `schedule(payload)` |
| Pause / resume | `pause()` / `resume()` |
| Hot swap | `snapshot()` on the predecessor, then `swap-in(state)` on the successor (`migrate(state)` first under `--policy migrator`) |

- **One export at a time, never during a frame turn.** A queued frame turn is served before a trigger, so a busy trigger caller cannot starve the mailbox.
- **Budgets.** Each export runs under `[budget].time_cap_seconds` and its own instruction fuel. An export that overruns yields the kernel's budget-exceeded row, and the session ends (the child is killed, `spirit.lifecycle.deadline` is journaled). A trigger that is queued behind another export and still cannot start in half its budget (for `telemetry` and `consolidate`, in half the pump's wall below, whichever is smaller) is dropped with a `maos: lifecycle trigger dropped` line: it is never a panic or a quarantine strike, and the pump counts it as neither a delivery nor a clean run, because the budget it missed was the busy session's. A dropped trigger never runs later.
- **Failure.** Every export returns `result<_, lifecycle-error>` (`declined(string)` or `failed(string)`); the dispatcher records a refusal as a panicked hook and applies its usual accounting. No export emits frames: a kernel trigger has no inbound intent lineage, so effects go through imports or the next frame turn. The hook ABI hands the guest no capability token.
- **Imports.** `idle`, `consolidate`, `telemetry` and `schedule` may call `recall`, `context` and `skills`, counted per export against the per-turn recall limit. `pause`, `resume`, `snapshot`, `swap-in` and `migrate` are import-free (`unavailable`), so a swap step never depends on the log at swap time.
- **Telemetry needs an explicit listing.** A component is delivered `telemetry` only when `[lifecycle].enabled_hooks` names `on_telemetry_event`; an empty list does not subscribe (unlike every other hook). The same `governance:telemetry-subscription:granted` / `…:revoked` rows are written. The pump delivers an event to its subscribers side by side and waits for each at most 95% of its `[budget].time_cap_seconds` or 5 s, whichever is smaller (an overrun is a budget overrun and quarantines it, and the row names the wall it hit). A subscriber whose telemetry used 80–95% of its budget in 3 of its last 5 deliveries is quarantined as well; that count is over telemetry only, and a quick delivery or a consolidation in between does not erase it. `on_consolidate` (the watchdog's pass and a forget's privacy notices) is held to 95% of the budget or 30 s the same way and runs side by side; a privacy notice still reaches a quarantined subscriber, but for at most that wall.
- **Schedules.** `[[schedule]]` entries are carried at admission for every form, so a component's schedule fires from its first start. Until the kernel landing of Story 20-9 (its second landing), a scheduled fire whose declared side-effect authority is refused still runs without a token.

### Hot swap, upgrade and cold swap

`maosctl spirit upgrade <id> --to <manifest>` swaps a component in place, whether `maos run` started it or the root booted it from an install. The successor must be a component too (`swap_form_mismatch`), must not declare any capability beyond what the operator admitted for this pid (`swap_capability_widened`), must leave `on_swap_in` enabled in its `[lifecycle].enabled_hooks` (`swap_hook_disabled`), and both versions must export `lifecycle` (`spawned_swap_unsupported`; `spawned_swap_not_ready` while the running component is still starting, so retry). Its artifact is resolved next to the target manifest and must stay inside that directory (`artifact_escapes_origin` for a symlink out of it). Every one of these refusals comes before anything is replaced, and the running version keeps serving. A hook that fires while a committed successor is waiting to be adopted is queued for it, not lost.

The successor is launched and probed first. The swap then waits for the turn boundary: an active turn finishes on the predecessor under its own leases, `snapshot` is taken, and the successor receives it through `swap-in`. The pid, every capability token the pid holds, the working memory and a pending halt survive; the predecessor's child is retired without revoking tokens. If the post-swap monitor reverts a committed swap, the predecessor's child is already gone, so the session ends with the journaled reason `swap_reverted_after_retire` and the Spirit is unloaded; nothing restores it automatically.

`--policy cold-swap` admits the successor at a new pid through the same admission path the daemon boots a component with: tokens are revoked by the unload and no state is carried. The admission returns only once the component has started and answered its lifecycle probe; one that is not a component, or fails to start, is unwound and refused (its FR11 archive record is written only after it started, so the predecessor is restored against its own record, even for a cross-major successor). A refused successor restores the predecessor from its retained manifest at another new pid (`cold_swap_reverted`). A `[cli_wrapper]` target is not a Spirit form and keeps `cli_wrapper_is_not_a_spirit_form`.
