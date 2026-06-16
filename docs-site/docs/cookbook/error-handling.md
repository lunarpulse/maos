---
title: Error Handling
sidebar_position: 13
description: Typed errors and recovery classes in MAOS Spirits.
---

# Error Handling

## Problem

Your Spirit encounters errors — a provider times out, a capability is revoked, an inference call returns invalid output, or a hot-swap migration fails. You need to handle these errors in a way the kernel understands, so it can apply the correct recovery policy (restart, escalate to operator, or trigger a halt).

## Solution

Use the typed error enums from `maos-domain` and `maos-spirit-abi` to communicate failure precisely:

```rust
use maos_spirit_abi::lifecycle::{Spirit, MigratorError, FramePayload};
use maos_spirit_abi::ctx::Ctx;

pub struct ResilientSpirit;

impl Spirit for ResilientSpirit {
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }

        match self.process_frame(payload) {
            Ok(()) => {}
            Err(SpiritError::Transient(msg)) => {
                // Transient errors: log and let the kernel retry via
                // the supervision watchdog. Don't panic.
                log_warning(&msg);
            }
            Err(SpiritError::Fatal(msg)) => {
                // Fatal errors: the Spirit cannot continue. Panic
                // triggers the [on_crash] policy (restart / stop).
                panic!("fatal: {msg}");
            }
            Err(SpiritError::InvalidInput(msg)) => {
                // Bad input from upstream: log and drop the frame.
                // Do NOT panic — bad input is the sender's problem.
                log_warning(&format!("dropping frame: {msg}"));
            }
        }
    }

    fn migrate(
        &self,
        _ctx: &mut Ctx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, MigratorError> {
        // Return typed migration errors the kernel can act on.
        if predecessor_state.is_empty() {
            return Err(MigratorError::DeserializationFailed(
                "empty predecessor state".into(),
            ));
        }

        let version = read_schema_version(predecessor_state);
        match version {
            Some(v) if v > 2 => Err(MigratorError::UnsupportedVersion(v)),
            Some(_) => do_migration(predecessor_state),
            None => Err(MigratorError::DeserializationFailed(
                "missing schema_version field".into(),
            )),
        }
    }
}

/// Spirit-internal error classification.
enum SpiritError {
    /// Retryable — network timeout, temporary provider unavailability.
    Transient(String),
    /// Unrecoverable — corrupt state, missing required configuration.
    Fatal(String),
    /// Bad input — malformed frame, invalid payload.
    InvalidInput(String),
}

impl ResilientSpirit {
    fn process_frame(&self, payload: &FramePayload) -> Result<(), SpiritError> {
        // Your processing logic here.
        Ok(())
    }
}

// Stubs for illustration.
fn log_warning(msg: &str) {}
fn read_schema_version(data: &[u8]) -> Option<u32> { None }
fn do_migration(data: &[u8]) -> Result<Vec<u8>, MigratorError> {
    Ok(data.to_vec())
}
```

Configure recovery policy in the manifest:

```toml
[on_crash]
action = "restart"                # restart | stop | notify_operator

[on_revocation]
action = "graceful_shutdown"      # graceful_shutdown | immediate_stop | notify_only

[supervision]
heartbeat_interval_ms = 5000
progress_threshold_ms = 30000
silent_failure_threshold_ms = 30000
```

## Discussion

MAOS error handling follows a layered model:

**Layer 1: Spirit-internal errors** — your code classifies errors into transient (retry-safe), fatal (panic), and invalid-input (drop and log). The kernel does not see this classification directly; it sees whether the hook returned normally or panicked.

**Layer 2: ABI-level typed errors** — the `MigratorError` enum is the one place the Spirit returns a typed error to the kernel. Its variants tell the kernel exactly what went wrong:

| Variant | Meaning | Kernel action |
|---|---|---|
| `NotImplemented` | Spirit does not support migration | Clean start (no state transfer) |
| `UnsupportedVersion(u32)` | Predecessor version too old/new | Fail admission with diagnostic |
| `DeserializationFailed(String)` | State blob is corrupt or unreadable | Fail swap, log to transparency log |
| `SerializationFailed(String)` | New state could not be encoded | Fail swap, log to transparency log |

**Layer 3: Kernel recovery policies** — the `[on_crash]` and `[on_revocation]` manifest sections declare what the kernel does when a Spirit fails:

- **`restart`** — the kernel restarts the Spirit with a fresh state (no hot-swap).
- **`stop`** — the kernel marks the Spirit as permanently failed.
- **`notify_operator`** — the kernel sends a notification and waits for human intervention.
- **`graceful_shutdown`** — on capability revocation, the kernel fires `on_unload` before stopping.
- **`immediate_stop`** — on revocation, the kernel stops immediately without cleanup.

**Best practices:**

- Never panic on bad input. Log it, drop the frame, and continue.
- Use `MigratorError` variants precisely — the kernel's diagnostics depend on them.
- Set `silent_failure_threshold_ms` to catch Spirits that swallow errors silently. If a Spirit neither processes frames nor panics for this duration, the kernel escalates.
- The error catalog at `/errors/` maps kernel error codes to human-readable explanations and remediation steps.
