<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `lifecycle` Module {#abi-lifecycle-module}

## Related {#abi-lifecycle-related}

- [ABI Stability Policy](/migrate/abi-stability) — when `ABI_VERSION` bumps
- [ctx Module](./ctx) — `Ctx` passed to every hook
- [cancellation Module](./cancellation) — `CancellationSignal` used in hooks


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

Lifecycle hooks trait — the in-process Rust trait contract between
the kernel and a Spirit.

Per architecture §5.3: "The Spirit ABI is the contract between the
kernel and a Spirit. Every Spirit conforms to it."

This module ships the **14-hook signature set** per FR55 (Story 5.2
extended from the original 11). The 1 deferred hook from the
architecture §5.3 14-hook list is:

| Hook | Deferred to | Reason |
|---|---|---|
| `epistemic_resolve` | Story 4.1 | Halt-protocol resolution |

Story 5.1 shipped the runtime firing for 11 hooks. Story 5.2 adds
the hot-swap hooks (`on_swap_out`, `snapshot`, `migrate`) with full
dispatcher integration, bringing the total to 14.

ADR-002 (Spirit form at v0.1): The trait signature serves both
`rust-inproc` and `subprocess` forms. The `CancellationSignal`
abstraction is the bridge that makes this possible — in-process
Spirits receive a `&dyn CancellationSignal` backed by the kernel's
Tokio runtime; subprocess Spirits receive a signal the wire protocol
carries as a message.


## Enums {#lifecycle-enums}

### `HookBudgetKey` {#maos-spirit-abi-lifecycle-hookbudgetkey}

Resource budget envelope key — the `#[hook(budget = "…")]`
attribute parses to one of these variants at compile time.

The kernel consults the manifest's `[budget]` section against this
key at firing time. Actual enforcement ships in Story 5.1.


```rust
pub enum HookBudgetKey {
    ContextWindow,
    TimeCapSeconds,
    CpuMaxPct,
    MemoryMaxMb,
    FdMax,
}
```

### `MigratorError` {#maos-spirit-abi-lifecycle-migratorerror}

Cross-major migration error — returned by `Spirit::migrate`.

`#[non_exhaustive]` lets future stories add variants without an ABI bump.
Hand-rolled Display impl because `maos-spirit-abi` is `#![no_std]`
with minimal dependencies (only `serde`).


```rust
pub enum MigratorError {
    NotImplemented,
    Malformed,
    Internal,
}
```


## Functions {#lifecycle-functions}

### `kernel_invocation_allowed` {#maos-spirit-abi-lifecycle-kernel-invocation-allowed}

Returns `true` if the kernel should invoke the given hook for a
Spirit whose manifest declares the `enabled_hooks` subset.

The invocation gate is signature-level: the kernel consults this
predicate at dispatch time. The runtime hook caller ships in
Story 5.1.

# Example {#maos-spirit-abi-lifecycle-kernel-invocation-allowed-example}

```rust
use maos_spirit_abi::lifecycle::kernel_invocation_allowed;

let enabled = &["on_load", "on_frame", "on_unload"];
assert!(kernel_invocation_allowed(enabled, "on_frame"));
assert!(!kernel_invocation_allowed(enabled, "on_idle"));
```


```rust
pub fn kernel_invocation_allowed(enabled_hooks: &[&str], hook_name: &str) -> bool
```


## Traits {#lifecycle-traits}

### `Spirit` {#maos-spirit-abi-lifecycle-spirit}

The Spirit lifecycle trait.

A Spirit implements this trait to receive lifecycle events from the
kernel. Every method has a default no-op body, so a Spirit author
writes only the hooks they care about.

# Example {#maos-spirit-abi-lifecycle-spirit-example}

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload};
use maos_spirit_abi::ctx::Ctx;

struct MySpirit;

impl Spirit for MySpirit {
    fn on_load(&self, _ctx: &mut Ctx) {
        // Initialize resources when admitted by the kernel.
    }

    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        let _data = &payload.frame_data[..payload.frame_len];
    }

    fn on_unload(&self, _ctx: &mut Ctx) {
        // Clean up resources before removal.
    }
}
```

# Firing semantics (architecture §5.3 references) {#maos-spirit-abi-lifecycle-spirit-firingsemantics-architecture5-3references}

| Hook | Fires when… | Payload | Implemented at |
|---|---|---|---|
| `on_load` | Spirit is admitted and loaded (§5.3.1) | — | Story 2.1 |
| `on_start` | Spirit receives first `Start` verb (§5.3.2) | — | Story 2.1 |
| `on_frame` | IAC frame arrives (§5.3.3) | `FramePayload` | Story 2.1 |
| `on_idle` | No frames for ≥ idle_timeout_ms (§5.3.4) | — | Story 2.1 |
| `on_telemetry_event` | Scalar-tap event fires (§5.3.5) | `TelemetryEventPayload` | Story 2.1 |
| `on_schedule` | Scheduled invocation fires (§5.3.6) | `SchedulePayload` | Story 2.1 |
| `on_swap_in` | Predecessor state arrives (§5.3.7) | `SwapInPayload` | Story 2.1 |
| `on_pause` | Kernel pauses Spirit (§5.3.8) | — | Story 2.1 |
| `on_resume` | Kernel resumes Spirit (§5.3.9) | — | Story 2.1 |
| `on_unload` | Spirit receives `Unload` verb (§5.3.10) | — | Story 2.1 |
| `on_consolidate` | Batch window closes (§5.3.11) | `ConsolidatePayload` | Story 2.1 |
| `on_swap_out` | Spirit is about to be swapped out (§5.3.12) | — | Story 5.2 ✅ |
| `snapshot` | Produce state snapshot for hot-swap (§5.3.13) | — (returns `Vec<u8>`) | Story 5.2 ✅ |
| `migrate` | Cross-major migration entry (§5.3.14) | predecessor_state `&[u8]` (returns `Result<Vec<u8>, MigratorError>`) | Story 5.2 ✅ |

All hooks receive a `&mut Ctx` carrying the cancellation signal,
capability handle, and mailbox handle.


```rust
pub trait Spirit {
    fn on_load(&self, ctx: &mut Ctx);
    fn on_start(&self, ctx: &mut Ctx);
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>);
    fn on_idle(&self, ctx: &mut Ctx);
    fn on_telemetry_event<'a>(&self, ctx: &mut Ctx, payload: &TelemetryEventPayload<'a>);
    fn on_schedule<'a>(&self, ctx: &mut Ctx, payload: &SchedulePayload<'a>);
    fn on_swap_in<'a>(&self, ctx: &mut Ctx, payload: &SwapInPayload<'a>);
    fn on_pause(&self, ctx: &mut Ctx);
    fn on_resume(&self, ctx: &mut Ctx);
    fn on_unload(&self, ctx: &mut Ctx);
    fn on_consolidate<'a>(&self, ctx: &mut Ctx, payload: &ConsolidatePayload<'a>);
    fn on_swap_out(&self, ctx: &mut Ctx);
    fn snapshot(&self, ctx: &mut Ctx) -> alloc::vec::Vec<u8>;
    fn migrate(&self, ctx: &mut Ctx, predecessor_state: &[u8]) -> Result<alloc::vec::Vec<u8>, MigratorError>;
}
```
