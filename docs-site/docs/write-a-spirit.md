---
title: Write a Spirit
sidebar_position: 1
description: The 30-minute first-Spirit path — scaffold, implement, test.
---

# Write a Spirit — the 30-minute first-Spirit path

Build your first MAOS Spirit with nothing more than this guide and `cargo generate`.

## 1. Scaffold from the official template

```bash
cargo generate --git https://github.com/lunarpulse/maos \
  templates/spirit-rust --name my-spirit
```

This pulls the Spirit template — the same one [`examples/example-spirit`](https://github.com/lunarpulse/maos/tree/main/examples/example-spirit) is baked from. You get a compiling Spirit with a lifecycle hook, a manifest, and a smoke test.

## 2. Implement one hook

Open `src/lib.rs` and edit `on_idle` (or another lifecycle hook). The `#[spirit]` proc-macro wires your `impl` block into the `SpiritVtable` the kernel calls.

See the [Manifest Schema](/manifest/latest) for every field your `spirit.toml` can declare, and the [ABI Reference](/abi/v1/) for the full hook surface.

## 3. Run it locally — no kernel required

The `LocalRunner` (and the fuller `SpiritTest` harness) fire your Spirit's hooks against a mock `Ctx` with zero kernel dependency:

```bash
cargo test -p my-spirit
```

## 4. What "done" means

Your first Spirit is done when it **compiles against the published ABI**, its smoke test passes, and it behaves correctly against the Butler-class regression scenarios.

## Next steps

- [Cookbook](/cookbook/) — 10+ runnable patterns for common Spirit tasks
- [Troubleshooting](/troubleshoot/) — every error code with cause and fix
- [Deploy](/deploy/) — air-gap, backup/restore, release signing
