---
title: Hot-Swap Migration
sidebar_position: 7
description: Implementing snapshot() and migrate() for zero-downtime state transfer between Spirit versions.
---

# Hot-Swap Migration

## Problem

You are deploying a new version of your Spirit and need to transfer in-flight state from the old instance to the new one without downtime. The kernel's hot-swap protocol calls `snapshot()` on the predecessor and `migrate()` on the successor — you need to implement both sides.

## Solution

Declare hot-swap support in the manifest:

```toml
[hot_swap]
state_schema_version = 2

[migrates_from]
versions = ["1.0.0"]

[halt_protocol_compatibility]
version = 1
```

Implement the hot-swap hooks:

```rust
use maos_spirit_abi::lifecycle::{Spirit, MigratorError, SwapInPayload};
use maos_spirit_abi::ctx::Ctx;

/// State envelope — versioned for forward compatibility.
#[derive(serde::Serialize, serde::Deserialize)]
struct StateSnapshot {
    schema_version: u32,
    counter: u64,
    buffer: Vec<u8>,
}

pub struct MySpirit {
    counter: std::cell::Cell<u64>,
    buffer: std::cell::RefCell<Vec<u8>>,
}

impl MySpirit {
    pub fn new() -> Self {
        Self {
            counter: std::cell::Cell::new(0),
            buffer: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl Spirit for MySpirit {
    /// Called on the OLD instance before swap-out.
    /// Flush in-flight work, release locks.
    fn on_swap_out(&self, ctx: &mut Ctx) {
        // Flush any pending writes before the kernel takes the snapshot.
    }

    /// Produce a CBOR-encoded state snapshot.
    /// The kernel passes this blob to the successor's on_swap_in / migrate.
    fn snapshot(&self, _ctx: &mut Ctx) -> Vec<u8> {
        let snap = StateSnapshot {
            schema_version: 2,
            counter: self.counter.get(),
            buffer: self.buffer.borrow().clone(),
        };
        // Use any codec — CBOR, bincode, JSON. The kernel treats it as
        // opaque bytes; the successor must understand the format.
        serde_json::to_vec(&snap).unwrap_or_default()
    }

    /// Called on the NEW instance when predecessor state arrives.
    fn on_swap_in<'a>(&self, ctx: &mut Ctx, payload: &SwapInPayload<'a>) {
        // on_swap_in receives the snapshot from the predecessor.
        // For same-version swaps, deserialise directly.
    }

    /// Cross-major migration: translate predecessor state to this version's schema.
    fn migrate(
        &self,
        _ctx: &mut Ctx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, MigratorError> {
        // Attempt to deserialise the predecessor's snapshot.
        let old: StateSnapshot = serde_json::from_slice(predecessor_state)
            .map_err(|e| MigratorError::DeserializationFailed(
                e.to_string().into()
            ))?;

        match old.schema_version {
            1 => {
                // Schema v1 -> v2: add the new buffer field.
                let migrated = StateSnapshot {
                    schema_version: 2,
                    counter: old.counter,
                    buffer: Vec::new(), // v1 had no buffer
                };
                serde_json::to_vec(&migrated)
                    .map_err(|e| MigratorError::SerializationFailed(
                        e.to_string().into()
                    ))
            }
            2 => {
                // Same schema — pass through.
                Ok(predecessor_state.to_vec())
            }
            v => Err(MigratorError::UnsupportedVersion(v)),
        }
    }
}
```

## Discussion

The hot-swap sequence the kernel orchestrates:

1. **`on_swap_out`** on the predecessor — flush state, release locks.
2. **`snapshot`** on the predecessor — produce a byte blob of serialised state.
3. **`on_swap_in`** on the successor — receive the blob for same-version swaps.
4. **`migrate`** on the successor — translate cross-version state (called only when `[migrates_from]` lists the predecessor version).

The `[hot_swap].state_schema_version` field is the version of the state blob your `snapshot()` produces. The `[migrates_from].versions` field declares which predecessor versions your `migrate()` can handle.

**Design rules:**

- Always version your snapshot format. Embed a `schema_version` field in the serialised blob so `migrate()` can branch on it.
- Return `MigratorError::NotImplemented` (the default) if your Spirit does not support migration. The kernel will fall back to a clean start.
- `MigratorError::UnsupportedVersion(v)` tells the kernel the predecessor version is too old to migrate — the operator gets a clear diagnostic.
- Keep snapshots small. The kernel holds the blob in memory during the swap window. Large state should be persisted externally with only a reference in the snapshot.
