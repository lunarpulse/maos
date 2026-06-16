---
title: Testing with the Spirit SDK
sidebar_position: 11
description: Using SpiritTest harness and LocalRunner to test Spirit hooks without a kernel.
---

# Testing with the Spirit SDK

## Problem

You want to test your Spirit's hook logic without standing up a full MAOS kernel. You need a mock `Ctx` that supplies a cancellation signal, capability handle, and mailbox handle — and a harness that fires hooks in the correct lifecycle order.

## Solution

Use the `Ctx::for_test()` constructor and write standard Rust tests:

```rust
#[cfg(test)]
mod tests {
    use maos_spirit_abi::lifecycle::Spirit;
    use maos_spirit_abi::ctx::Ctx;
    use maos_spirit_abi::cancellation::NeverCancel;

    use super::MySpirit;

    #[test]
    fn on_idle_increments_counter() {
        let spirit = MySpirit::new();

        // Ctx::for_test() builds a mock context with NeverCancel,
        // a zeroed CapabilityHandle, and a zeroed MailboxHandle.
        let mut ctx = Ctx::for_test();

        spirit.on_load(&mut ctx);
        spirit.on_start(&mut ctx);

        // Fire on_idle and assert side effects.
        spirit.on_idle(&mut ctx);
        assert_eq!(spirit.idle_count(), 1);
    }

    #[test]
    fn on_frame_processes_payload() {
        use maos_spirit_abi::lifecycle::FramePayload;

        let spirit = MySpirit::new();
        let mut ctx = Ctx::for_test();
        spirit.on_load(&mut ctx);

        let payload = FramePayload {
            data: b"hello",
            _marker: core::marker::PhantomData,
        };

        spirit.on_frame(&mut ctx, &payload);
        assert_eq!(spirit.frames_processed(), 1);
    }

    #[test]
    fn snapshot_round_trips() {
        let spirit = MySpirit::new();
        let mut ctx = Ctx::for_test();
        spirit.on_load(&mut ctx);

        // Mutate some state.
        spirit.on_idle(&mut ctx);
        spirit.on_idle(&mut ctx);

        // Snapshot and verify it deserialises.
        let snap = spirit.snapshot(&mut ctx);
        assert!(!snap.is_empty(), "snapshot should contain state");

        // Verify migration from the snapshot.
        let successor = MySpirit::new();
        let migrated = successor.migrate(&mut ctx, &snap);
        assert!(migrated.is_ok(), "same-version migration should succeed");
    }

    #[test]
    fn cancellation_aborts_early() {
        use maos_spirit_abi::cancellation::CancellationSignal;

        // A custom signal that is always cancelled.
        struct AlwaysCancel;
        impl CancellationSignal for AlwaysCancel {
            fn is_cancelled(&self) -> bool { true }
        }

        let spirit = MySpirit::new();
        // Build a Ctx with the AlwaysCancel signal.
        let mut ctx = Ctx::for_test_with_cancellation(
            &AlwaysCancel as &'static dyn CancellationSignal,
        );

        spirit.on_idle(&mut ctx);
        // Spirit should have bailed early — no work done.
        assert_eq!(spirit.idle_count(), 0);
    }
}
```

For integration-level tests, use the `LocalRunner` which orchestrates the full lifecycle sequence:

```rust
#[cfg(test)]
mod integration {
    use maos_spirit_sdk::spirit_test::LocalRunner;
    use super::MySpirit;

    #[test]
    fn full_lifecycle_smoke() {
        let mut runner = LocalRunner::new(MySpirit::new());

        // LocalRunner fires hooks in kernel order:
        // on_load → on_start → (frames/idle/schedule) → on_unload
        runner.load();
        runner.start();
        runner.idle();       // fires on_idle
        runner.unload();     // fires on_unload

        assert!(runner.completed_cleanly());
    }
}
```

## Discussion

The Spirit SDK provides two testing surfaces:

**1. `Ctx::for_test()` — unit-level mock context**

- Uses `NeverCancel` as the cancellation signal (never fires).
- Provides zeroed `CapabilityHandle(0)` and `MailboxHandle(0)`.
- No async runtime required — pure synchronous testing.
- Useful for testing individual hook logic in isolation.

**2. `LocalRunner` — integration-level lifecycle harness**

- Fires hooks in the same order the kernel does.
- Validates that hooks do not panic and return within timeout.
- Does not require a running kernel or network access.
- Useful for smoke-testing the full lifecycle sequence.

**Testing patterns:**

- **Test each hook independently** with `Ctx::for_test()` before testing the full lifecycle.
- **Test cancellation** by providing a custom `CancellationSignal` implementation that returns `true`.
- **Test hot-swap** by calling `snapshot()` on one instance and `migrate()` on another.
- **Test error paths** by feeding malformed payloads and asserting the Spirit handles them gracefully.

The `NeverCancel` implementation is intentionally minimal — it exists for testing, not production. In production, the kernel provides a `TokioCancellationSignal` backed by `Arc<AtomicBool>`.
