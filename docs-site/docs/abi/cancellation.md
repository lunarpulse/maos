---
title: cancellation
sidebar_position: 6
description: "CancellationSignal trait and NeverCancel — runtime-agnostic cancellation for Spirit hooks."
---

# `cancellation` Module

The cancellation module provides a `#![no_std]` abstraction for Spirit hook cancellation. It bridges the gap between in-process Spirits (backed by Tokio) and subprocess Spirits (backed by wire protocol signals) per ADR-002.

Introduced in Story 2.1.

## CancellationSignal Trait

The kernel-side bridge that lets hook implementations check or await cancellation without coupling to a specific runtime.

```rust
pub trait CancellationSignal {
    /// Synchronous poll: returns true if cancellation has been requested.
    fn is_cancelled(&self) -> bool;

    /// Async await: returns a future that resolves when cancellation is requested.
    /// Not object-safe — use is_cancelled() on &dyn CancellationSignal.
    fn cancelled(&self) -> CancellationFuture<'_>
    where
        Self: Sized;
}
```

### Methods

| Method | Returns | Object-safe? | Description |
|---|---|---|---|
| `is_cancelled()` | `bool` | ✅ Yes | Synchronous poll — use in hook implementations |
| `cancelled()` | `CancellationFuture` | ❌ No (`Self: Sized`) | Async wait — override in production adapters |

### Example: Checking Cancellation in a Hook

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload};
use maos_spirit_abi::ctx::Ctx;

struct BatchProcessor;

impl Spirit for BatchProcessor {
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        // Check cancellation before expensive work
        if ctx.cancellation().is_cancelled() {
            return;
        }

        // Process the frame
        let data = &payload.frame_data[..payload.frame_len];
        // ... batch processing logic ...
    }
}
```

## CancellationFuture

Future returned by `CancellationSignal::cancelled()`. Polls `is_cancelled()` without registering a waker.

```rust
pub struct CancellationFuture<'a> {
    signal: &'a dyn CancellationSignal,
}

impl<'a> Future for CancellationFuture<'a> {
    type Output = ();
    // Returns Poll::Ready(()) when is_cancelled() is true
}
```

**Important limitation:** The default `cancelled()` implementation does not register a waker. Executors relying solely on waker notifications (including Tokio) will never re-poll after the first `Pending`. Use `is_cancelled()` for synchronous polling in hooks. Production adapters like `TokioCancellationSignal` override `cancelled()` with an efficient runtime-aware implementation.

## NeverCancel

Reference implementation that never fires. Useful for tests and SDK-side unit tests that do not require an async runtime.

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverCancel;

impl CancellationSignal for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}
```

### Example: Using NeverCancel in Tests

```rust
use maos_spirit_abi::cancellation::{CancellationSignal, NeverCancel};

let signal = NeverCancel;
assert!(!signal.is_cancelled());

// NeverCancel is the default signal used by Ctx::mock()
```

## Implementation Guide

To implement a production cancellation signal (e.g., backed by Tokio), implement the trait and override `cancelled()`:

```rust
use maos_spirit_abi::cancellation::CancellationSignal;

struct TokioCancellationSignal {
    // Backed by tokio_util::sync::CancellationToken
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationSignal for TokioCancellationSignal {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Acquire)
    }
    // Override cancelled() with a Tokio-aware async wait in production
}
```
