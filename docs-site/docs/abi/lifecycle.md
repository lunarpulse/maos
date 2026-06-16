---
title: lifecycle
sidebar_position: 2
description: Spirit trait with 14 lifecycle hooks, SpiritVtable dispatch table, and payload types.
---

# `lifecycle` Module

The lifecycle module defines the **Spirit trait** — the in-process Rust trait contract between the kernel and a Spirit. It provides 14 lifecycle hooks (per FR55), a dispatch vtable, and typed payloads.

Introduced in Story 2.1 (11 hooks), extended in Story 5.2 (+3 hot-swap hooks: `on_swap_out`, `snapshot`, `migrate`).

## Spirit Trait

The core trait every Spirit implements. All hooks have default no-op bodies — a Spirit author writes only the hooks they need.

```rust
pub trait Spirit {
    fn on_load(&self, ctx: &mut Ctx) {}
    fn on_start(&self, ctx: &mut Ctx) {}
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {}
    fn on_idle(&self, ctx: &mut Ctx) {}
    fn on_telemetry_event<'a>(&self, ctx: &mut Ctx, payload: &TelemetryEventPayload<'a>) {}
    fn on_schedule<'a>(&self, ctx: &mut Ctx, payload: &SchedulePayload<'a>) {}
    fn on_swap_in<'a>(&self, ctx: &mut Ctx, payload: &SwapInPayload<'a>) {}
    fn on_pause(&self, ctx: &mut Ctx) {}
    fn on_resume(&self, ctx: &mut Ctx) {}
    fn on_unload(&self, ctx: &mut Ctx) {}
    fn on_consolidate<'a>(&self, ctx: &mut Ctx, payload: &ConsolidatePayload<'a>) {}
    fn on_swap_out(&self, ctx: &mut Ctx) {}                                          // Story 5.2
    fn snapshot(&self, ctx: &mut Ctx) -> Vec<u8> { Vec::new() }                      // Story 5.2
    fn migrate(&self, ctx: &mut Ctx, predecessor_state: &[u8])
        -> Result<Vec<u8>, MigratorError> { Err(MigratorError::NotImplemented) }     // Story 5.2
}
```

### Hook Reference

| Hook | Fires When | Payload | §5.3 Ref |
|---|---|---|---|
| `on_load` | Spirit admitted and loaded | — | §5.3.1 |
| `on_start` | First `Start` verb received | — | §5.3.2 |
| `on_frame` | IAC frame arrives at mailbox | `FramePayload` | §5.3.3 |
| `on_idle` | No frames for ≥ `idle_timeout_ms` | — | §5.3.4 |
| `on_telemetry_event` | Scalar-tap event fires | `TelemetryEventPayload` | §5.3.5 |
| `on_schedule` | Scheduled invocation fires | `SchedulePayload` | §5.3.6 |
| `on_swap_in` | Predecessor state arrives (hot-swap) | `SwapInPayload` | §5.3.7 |
| `on_pause` | Kernel pauses the Spirit | — | §5.3.8 |
| `on_resume` | Kernel resumes a paused Spirit | — | §5.3.9 |
| `on_unload` | `Unload` verb received | — | §5.3.10 |
| `on_consolidate` | Batch window closes | `ConsolidatePayload` | §5.3.11 |
| `on_swap_out` | About to be swapped out | — | §5.3.12 |
| `snapshot` | Produce state snapshot for hot-swap | — (returns `Vec<u8>`) | §5.3.13 |
| `migrate` | Cross-major migration entry | predecessor `&[u8]` (returns `Result<Vec<u8>, MigratorError>`) | §5.3.14 |

### Example: Implementing a Spirit

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload};
use maos_spirit_abi::ctx::Ctx;

struct MySpirit {
    frame_count: std::sync::atomic::AtomicU64,
}

impl Spirit for MySpirit {
    fn on_load(&self, ctx: &mut Ctx) {
        // Initialize resources when admitted by the kernel
    }

    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        // Check for cancellation before processing
        if ctx.cancellation().is_cancelled() {
            return;
        }
        self.frame_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Process payload.frame_data[..payload.frame_len]
    }

    fn on_unload(&self, ctx: &mut Ctx) {
        // Clean up resources before removal
    }
}
```

## Payload Types

All payloads are `#[derive(Debug, Clone, Copy)]` and carry raw byte slices. Full typed frames (IAC Bus dispatch) land in Epic 6.

### FramePayload

```rust
pub struct FramePayload<'a> {
    pub frame_data: &'a [u8],
    pub frame_len: usize,
}
```

### TelemetryEventPayload

```rust
pub struct TelemetryEventPayload<'a> {
    pub event_data: &'a [u8],
    pub event_len: usize,
}
```

### SchedulePayload

```rust
pub struct SchedulePayload<'a> {
    pub schedule_data: &'a [u8],
    pub schedule_len: usize,
}
```

### SwapInPayload

```rust
pub struct SwapInPayload<'a> {
    pub predecessor_state: &'a [u8],
    pub state_len: usize,
}
```

### ConsolidatePayload

```rust
pub struct ConsolidatePayload<'a> {
    pub batch_data: &'a [u8],
    pub batch_len: usize,
}
```

## HookBudgetKey

Maps `#[hook(budget = "...")]` attributes to resource budget envelopes at compile time. The kernel consults the manifest's `[budget]` section against this key at firing time.

```rust
pub enum HookBudgetKey {
    ContextWindow,   // Token or byte-equivalent context window size
    TimeCapSeconds,  // Per-invocation time cap
    CpuMaxPct,       // CPU usage percentage cap
    MemoryMaxMb,     // Memory ceiling in MB
    FdMax,           // File descriptor cap
}
```

## MigratorError

Returned by `Spirit::migrate` when cross-major migration fails.

```rust
#[non_exhaustive]
pub enum MigratorError {
    NotImplemented,
    Malformed(String),
    Internal(String),
}
```

Use the constructor methods to enforce non-empty messages:

```rust
// Preferred — enforces non-empty message
let err = MigratorError::new_malformed("missing header field");
let err = MigratorError::new_internal("state checksum mismatch");
```

## SpiritVtable

Per-hook function-pointer dispatch table used by the kernel at runtime (Story 5.1). The vtable is `#[repr(C)]` for subprocess-form FFI dispatch (Epic 5, Story 5.5x).

```rust
#[repr(C)]
pub struct SpiritVtable<T: Spirit + 'static> {
    // One function pointer per hook — constructed by the #[spirit] proc-macro
}
```

### Example: Constructing a vtable

```rust
use maos_spirit_abi::lifecycle::{Spirit, SpiritVtable};

struct EchoSpirit;

impl Spirit for EchoSpirit {
    // Use default no-op implementations
}

// The #[spirit] proc-macro generates this; manual construction for illustration:
let vtable = SpiritVtable::<EchoSpirit>::new();
```

## `kernel_invocation_allowed`

Predicate function the kernel calls at dispatch time to check if a hook is in the Spirit's `enabled_hooks` subset.

```rust
pub fn kernel_invocation_allowed(enabled_hooks: &[&str], hook_name: &str) -> bool;
```

### Example

```rust
use maos_spirit_abi::lifecycle::kernel_invocation_allowed;

let enabled = &["on_load", "on_frame", "on_unload"];
assert!(kernel_invocation_allowed(enabled, "on_frame"));
assert!(!kernel_invocation_allowed(enabled, "on_idle"));
```
