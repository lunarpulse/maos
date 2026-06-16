---
title: deprecation
sidebar_position: 8
description: "DeprecationWarning type — deprecation channel for tracking ABI surface evolution."
---

# `deprecation` Module

The deprecation module provides the `DeprecationWarning` type — the channel through which Spirit code is notified about deprecated ABI surfaces it has used. Observable via `Ctx::deprecation_warnings()`.

Introduced in Story 7.1 (v0.5 binding). At v0.5, the ABI has **zero deprecations** — the channel ships empty-present to establish the infrastructure for future deprecation cycles.

## DeprecationWarning

A deprecation warning observable from `Ctx::deprecation_warnings()`. Populated by the kernel at hook-fire time from any ABI surface annotated with `#[maos_attrs::deprecated_since(...)]`.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeprecationWarning {
    /// The deprecated surface identifier — e.g., "Ctx::old_send_method".
    pub surface: &'static str,
    /// The version the surface was deprecated in — e.g., "0.5".
    pub since_version: &'static str,
    /// The version the surface is planned for removal in — e.g., "1.0".
    pub planned_removal: &'static str,
    /// Migration hint — e.g., "use Ctx::new_send_method instead".
    pub migration_hint: &'static str,
}
```

### Constructor

```rust
impl DeprecationWarning {
    pub const fn new(
        surface: &'static str,
        since_version: &'static str,
        planned_removal: &'static str,
        migration_hint: &'static str,
    ) -> Self;
}
```

The constructor is `const fn`, allowing deprecation warnings to be defined as compile-time constants.

### Example: Creating and Observing Warnings

```rust
use maos_spirit_abi::DeprecationWarning;

// Define a deprecation warning (typically done by the kernel)
const OLD_SEND_WARNING: DeprecationWarning = DeprecationWarning::new(
    "Ctx::old_send_method",
    "0.5",
    "1.0",
    "use Ctx::new_send_method instead",
);

assert_eq!(OLD_SEND_WARNING.surface, "Ctx::old_send_method");
assert_eq!(OLD_SEND_WARNING.since_version, "0.5");
assert_eq!(OLD_SEND_WARNING.planned_removal, "1.0");
```

### Example: Checking for Deprecations in a Hook

```rust
use maos_spirit_abi::lifecycle::Spirit;
use maos_spirit_abi::ctx::Ctx;

struct CarefulSpirit;

impl Spirit for CarefulSpirit {
    fn on_start(&self, ctx: &mut Ctx) {
        // Check if any deprecated surfaces were used
        let warnings = ctx.deprecation_warnings();
        if !warnings.is_empty() {
            for w in warnings {
                // Log: "{surface} deprecated since {since_version},
                //        removal planned at {planned_removal}.
                //        Migration: {migration_hint}"
            }
        }
    }
}
```

### Example: Testing with Mock Warnings

The `spirit-test` SDK uses `Ctx::mock_with_deprecation_warnings()` to verify the deprecation channel works even though v0.5 has no real deprecations:

```rust
#[cfg(test)]
fn test_deprecation_channel() {
    use maos_spirit_abi::ctx::Ctx;
    use maos_spirit_abi::DeprecationWarning;

    let warning = DeprecationWarning::new(
        "Ctx::legacy_api",
        "0.5",
        "1.0",
        "use Ctx::modern_api instead",
    );
    let ctx = Ctx::mock_with_deprecation_warnings(vec![warning.clone()]);

    assert_eq!(ctx.deprecation_warnings().len(), 1);
    assert_eq!(ctx.deprecation_warnings()[0].surface, "Ctx::legacy_api");
}
```

## Deprecation Lifecycle

Per NFR-Maint-5, deprecated surfaces follow this timeline:

1. **Annotated**: Surface receives `#[maos_attrs::deprecated_since(version = "X.Y", remove_at = "X+2.Y", migration = "...")]`
2. **Warning period**: 2 minor releases — `DeprecationWarning` emitted at hook-fire time
3. **Removal**: 1 major release — surface is removed, compile error for callers

The `stability-matrix --check` CI gate enforces that every `#[deprecated_since]` annotation has a corresponding `STABILITY.md` entry and `BREAKING.md` record.

## Re-export

`DeprecationWarning` is re-exported at the crate root:

```rust
use maos_spirit_abi::DeprecationWarning;
// equivalent to:
use maos_spirit_abi::deprecation::DeprecationWarning;
```
