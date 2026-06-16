---
title: Cancellation Handling
sidebar_position: 5
description: Checking the cancellation signal in long-running Spirit hooks to bail early.
---

# Cancellation Handling

## Problem

Your Spirit hook does long-running work — iterating over a large dataset, making multiple inference calls, or running a multi-step pipeline. If the kernel needs to unload or swap the Spirit, the hook must bail out promptly. Ignoring the cancellation signal causes the kernel's supervision watchdog to escalate to a forced kill after `progress_threshold_ms`.

## Solution

Poll `ctx.cancellation().is_cancelled()` at natural yield points in your hook:

```rust
use maos_spirit_abi::lifecycle::{Spirit, FramePayload};
use maos_spirit_abi::ctx::Ctx;

pub struct BatchProcessor;

impl Spirit for BatchProcessor {
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        let items = parse_work_items(payload);

        for (i, item) in items.iter().enumerate() {
            // Check cancellation before each unit of work.
            if ctx.cancellation().is_cancelled() {
                // Optionally persist progress so a successor can resume.
                save_checkpoint(i);
                return;
            }
            process_item(item);
        }
    }

    fn on_idle(&self, ctx: &mut Ctx) {
        // Even short hooks should check cancellation at the top.
        if ctx.cancellation().is_cancelled() {
            return;
        }
        run_compaction();
    }
}

// Stubs for illustration.
fn parse_work_items(_payload: &FramePayload) -> Vec<WorkItem> { vec![] }
fn process_item(_item: &WorkItem) {}
fn save_checkpoint(_index: usize) {}
fn run_compaction() {}
struct WorkItem;
```

## Discussion

The `CancellationSignal` trait abstracts cancellation across Spirit forms:

- **In-process Spirits** receive a signal backed by the kernel's Tokio runtime (an `Arc<AtomicBool>` under the hood).
- **Subprocess Spirits** receive the signal via the wire protocol as a message.

The abstraction means your code works identically regardless of form. The trait exposes two surfaces:

1. **`is_cancelled() -> bool`** — synchronous poll; use this in loop bodies and at the top of hooks.
2. **`cancelled() -> impl Future`** — async wait; useful in production adapters (`TokioCancellationSignal`) but the default implementation does not register a waker, so prefer `is_cancelled()` in hook code.

**When to check:**

- At the top of every hook, before doing any work.
- Inside loops, before each iteration (or every N iterations for tight loops).
- Before and after any I/O or inference call.

**What happens if you ignore it:**

The kernel's supervision watchdog tracks hook progress. If a hook does not return within `progress_threshold_ms` (default 30 000 ms) after cancellation, the kernel escalates — first a forced task abort, then a full Spirit unload. Checking cancellation is not optional for production Spirits.

In tests, the SDK provides `NeverCancel` — a reference implementation that always returns `false` — so your unit tests can exercise hook logic without requiring an async runtime.
