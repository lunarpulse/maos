---
title: ctx
sidebar_position: 3
description: "Ctx type — Spirit-author-facing context carrying cancellation, capability, and mailbox handles."
---

# `ctx` Module

The `ctx` module provides the `Ctx` type — the Spirit-author-facing context passed to every lifecycle hook. It is the **only** surface through which a hook can interact with the kernel (per invariant I1: Spirits cannot bypass the Capability Registry).

Introduced in Story 2.1.

## Ctx

```rust
pub struct Ctx {
    // Internal fields — not directly accessible
    cancellation: &'static dyn CancellationSignal,
    capability_handle: CapabilityHandle,
    mailbox_handle: MailboxHandle,
    deprecation_warnings: Vec<DeprecationWarning>,
}
```

### Methods

| Method | Returns | Description |
|---|---|---|
| `cancellation()` | `&dyn CancellationSignal` | Borrow the cancellation signal for checking/awaiting cancellation |
| `capability()` | `CapabilityHandle` | Opaque handle resolved by the kernel at capability mediation time |
| `mailbox()` | `MailboxHandle` | Opaque handle resolved by the kernel at IAC dispatch time |
| `deprecation_warnings()` | `&[DeprecationWarning]` | Observe deprecated ABI surfaces used during the current hook fire (Story 7.1) |

### Example: Using Ctx in a Hook

```rust
use maos_spirit_abi::lifecycle::Spirit;
use maos_spirit_abi::ctx::Ctx;

struct MySpirit;

impl Spirit for MySpirit {
    fn on_start(&self, ctx: &mut Ctx) {
        // Check for cancellation
        if ctx.cancellation().is_cancelled() {
            return;
        }

        // Read the capability handle for SDK calls
        let cap = ctx.capability();

        // Read the mailbox handle for IAC frame operations
        let mbox = ctx.mailbox();

        // Check for deprecation warnings (empty at v0.5)
        for warning in ctx.deprecation_warnings() {
            // Log deprecated surface usage
        }
    }
}
```

## CapabilityHandle

Opaque handle to a capability token held kernel-side. The Spirit sees only this integer handle; the kernel resolves it to the actual `CapabilityToken` at mediation time.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityHandle(pub u64);
```

### Example

```rust
use maos_spirit_abi::ctx::CapabilityHandle;

let handle = CapabilityHandle(42);
assert_eq!(handle, CapabilityHandle(42));
```

## MailboxHandle

Opaque handle to the Spirit's mailbox. The Spirit sees only this integer handle; the kernel resolves it to the actual mailbox queue at dispatch time.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxHandle(pub u64);
```

### Example

```rust
use maos_spirit_abi::ctx::MailboxHandle;

let handle = MailboxHandle(7);
assert_eq!(handle, MailboxHandle(7));
```

## Mock Constructors (Test Only)

Available under `#[cfg(any(test, feature = "mock"))]`:

### `Ctx::mock()`

Constructs a `Ctx` with `NeverCancel`, zero handles, and no deprecation warnings. For SDK-side unit tests.

```rust
#[cfg(test)]
fn test_my_spirit() {
    use maos_spirit_abi::ctx::Ctx;
    use maos_spirit_abi::lifecycle::Spirit;

    let mut ctx = Ctx::mock();
    let spirit = MySpirit;
    spirit.on_load(&mut ctx);
}
```

### `Ctx::mock_with_deprecation_warnings()`

Constructs a mock `Ctx` with pre-populated deprecation warnings. Used by `spirit-test` to verify the deprecation channel (Story 7.1).

```rust
#[cfg(test)]
fn test_deprecation_surface() {
    use maos_spirit_abi::ctx::Ctx;
    use maos_spirit_abi::DeprecationWarning;

    let warning = DeprecationWarning::new(
        "Ctx::old_method",
        "0.5",
        "1.0",
        "use Ctx::new_method instead",
    );
    let ctx = Ctx::mock_with_deprecation_warnings(vec![warning]);
    assert_eq!(ctx.deprecation_warnings().len(), 1);
}
```

### `Ctx::for_rust_inproc_hook()`

Kernel-internal constructor for rust-inproc dispatch (NOT gated behind `mock`). Spirit authors never call this directly.

```rust
// Kernel-side only — not for Spirit authors
let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
```
