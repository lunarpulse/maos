<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `cancellation` Module {#abi-cancellation-module}

## Related {#abi-cancellation-related}

- [ADR-002](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-002-spirit-form-at-v01-subprocess-only-inproc-gated-on-measurement.md) — why cancellation is runtime-agnostic
- [lifecycle Module](./lifecycle) — hooks that poll `CancellationSignal`
- [ctx Module](./ctx) — `Ctx::cancellation()`


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

Cancellation signal trait — no_std abstraction for Spirit hook cancellation.

ADR-002 commits to subprocess-form Spirits as the v0.1 default with
rust-inproc gated on measurement. Subprocess Spirits live in a different
process from the kernel's Tokio runtime; they cannot directly reference a
`tokio_util::sync::CancellationToken` instance. This trait provides an
abstraction the wire protocol can carry as a signal.

The SDK side (std-aware, tokio-aware) provides `TokioCancellationSignal`
as the production impl; tests use `NeverCancel`.


## Structs {#cancellation-structs}

### `CancellationFuture` {#maos-spirit-abi-cancellation-cancellationfuture}

Future returned by [`CancellationSignal::cancelled`].

**Limitation:** The default implementation polls `is_cancelled()` without
registering a waker. Executors that rely solely on waker notifications
(including Tokio) will never re-poll after the first `Pending` result,
causing this future to hang indefinitely if the signal is not already
cancelled. Use `is_cancelled()` for synchronous polling in hook
implementations; the async `cancelled()` method is a placeholder for
production adapters (`TokioCancellationSignal`) that override it with
an efficient runtime-aware wait.


```rust
pub struct CancellationFuture<'a> { /* private fields */ }
```

### `NeverCancel` {#maos-spirit-abi-cancellation-nevercancel}

Reference implementation: a cancellation signal that never fires.

Useful for trait-object dispatch smoke tests and SDK-side unit tests
that do not require an async runtime.

# Example {#maos-spirit-abi-cancellation-nevercancel-example}

```rust
use maos_spirit_abi::cancellation::{CancellationSignal, NeverCancel};

let signal = NeverCancel;
assert!(!signal.is_cancelled());

// NeverCancel is the default signal used by Ctx::mock().
```


```rust
pub struct NeverCancel;
```


## Traits {#cancellation-traits}

### `CancellationSignal` {#maos-spirit-abi-cancellation-cancellationsignal}

Trait for cancellation signals — the kernel-side bridge that lets
hook implementations check or await cancellation without coupling
to a specific runtime.

Structurally parallel to the D9 pattern from Story 1b.6:
one no_std wire-format abstraction, one std operational adapter.

# Example {#maos-spirit-abi-cancellation-cancellationsignal-example}

```ignore
fn on_frame(&self, ctx: &mut Ctx) {
    if ctx.cancellation().is_cancelled() {
        return; // bail early
    }
    // … do work …
}
```


```rust
pub trait CancellationSignal {
    fn is_cancelled(&self) -> bool;
}
```
