<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `ctx` Module {#abi-ctx-module}

## Related {#abi-ctx-related}

- [lifecycle Module](./lifecycle) — hooks that receive `Ctx`
- [cancellation Module](./cancellation) — `CancellationSignal` returned by `Ctx::cancellation()`
- [deprecation Module](./deprecation) — `DeprecationWarning` returned by `Ctx::deprecation_warnings()`


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

Spirit-author-facing context type.

`Ctx` carries the cancellation signal, capability handle, and
mailbox handle a Spirit hook receives at invocation time. It is the
ONLY surface through which a hook can interact with the kernel —
per I1, Spirits cannot bypass the Capability Registry.


## Structs {#ctx-structs}

### `Ctx` {#maos-spirit-abi-ctx-ctx}

Spirit-author-facing context passed to every hook.

Carries a cancellation signal (so long-running hooks can bail early),
an opaque capability handle (so hooks can use capability APIs
via the SDK), and an opaque mailbox handle (for IAC frame send/receive
via the SDK).

The cancellation signal is stored with a `'static` bound because
the kernel owns the underlying signal (e.g., an `Arc<AtomicBool>`)
and ensures it outlives all Spirit hook invocations. This avoids
lifetime parameters on the vtable dispatch functions.


```rust
pub struct Ctx { /* private fields */ }
```

### Inherent Items {#maos-spirit-abi-ctx-ctx-inherent-items}

Methods and associated functions implemented directly on this type.

### `cancellation` {#cancellation}

Borrow the cancellation signal.

# Example {#cancellation-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let mut ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert!(!ctx.cancellation().is_cancelled());
```


```rust
pub fn cancellation(&self) -> &dyn CancellationSignal
```

### `capability` {#capability}

Opaque capability handle — the kernel resolves this at mediation time.

# Example {#capability-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert_eq!(ctx.capability(), CapabilityHandle(100));
```


```rust
pub fn capability(&self) -> CapabilityHandle
```

### `mailbox` {#mailbox}

Opaque mailbox handle — the kernel resolves this at IAC dispatch time.

# Example {#mailbox-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert_eq!(ctx.mailbox(), MailboxHandle(200));
```


```rust
pub fn mailbox(&self) -> MailboxHandle
```

### `deprecation_warnings` {#deprecation-warnings}

Story 7.1 v0.5 binding — observe any deprecated ABI surfaces the
Spirit code has used during the current hook fire. Returns an empty
slice at v0.5 because the v0.5 ABI has no deprecations.

# Example {#deprecation-warnings-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert!(ctx.deprecation_warnings().is_empty());
```


```rust
pub fn deprecation_warnings(&self) -> &[DeprecationWarning]
```

### `for_rust_inproc_hook` {#for-rust-inproc-hook}

Construct a kernel-internal `Ctx` for rust-inproc hook dispatch.

Story 5.1 closure: the kernel-side `HookDispatcher` requires a
`Ctx` to pass to each hook fire. At v0.3-β the rust-inproc form
uses a `&'static NeverCancel` because the kernel mediates
cancellation through `KernelCtx`'s SCB state, not through `Ctx`.
Real handles are zero-valued (rust-inproc form does not use them;
Story 5.5x's subprocess form will populate them from the wire
decode handshake).

This constructor is NOT gated behind the `mock` feature — it is
the production-supported kernel-side surface for rust-inproc
dispatch, and is callable only from within `maos-kernel-core`
(no Spirit author can call this; the Spirit receives a fully-
constructed `Ctx` from the kernel and never constructs one
itself).

# Example {#for-rust-inproc-hook-example}

```rust
use maos_spirit_abi::ctx::{Ctx, CapabilityHandle, MailboxHandle};

let ctx = Ctx::for_rust_inproc_hook(
    CapabilityHandle(100),
    MailboxHandle(200),
);
assert_eq!(ctx.capability(), CapabilityHandle(100));
assert_eq!(ctx.mailbox(), MailboxHandle(200));
```


```rust
pub fn for_rust_inproc_hook(capability_handle: CapabilityHandle, mailbox_handle: MailboxHandle) -> Self
```
