---
title: Hello-World Spirit
sidebar_position: 2
description: Build a minimal MAOS Spirit with a single on_idle hook in about 30 minutes.
---

# Hello-World Spirit

## Problem

You want the fastest path from zero to a running Spirit. You need a project that compiles against the published ABI, implements one hook, and passes its smoke test — all in about 30 minutes.

## Solution

Scaffold the project from the official template:

```bash
cargo generate --git https://github.com/lunarpulse/maos \
  templates/spirit-rust --name hello-spirit
```

Add a minimal manifest at `spirit.toml`:

```toml
[class]
name = "hello-spirit"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"
description = "A minimal hello-world Spirit."

[author]
name = "you"

[sandbox]
tier = "baseline"

[resources]
max_memory_mb = 64
max_cpu_ms = 1000

[budget]
max_inference_calls = 0
time_cap_seconds = 60
```

Implement the Spirit trait in `src/lib.rs`:

```rust
use maos_spirit_abi::lifecycle::Spirit;
use maos_spirit_abi::ctx::Ctx;

pub struct HelloSpirit;

impl Spirit for HelloSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        // Called when no inbound frames arrive for >= idle_timeout_ms.
        // This is where a minimal Spirit does its work.
        if ctx.cancellation().is_cancelled() {
            return;
        }
        // Your logic here — log, emit a frame, update state, etc.
    }
}
```

Run the smoke test:

```bash
cargo test -p hello-spirit
```

## Discussion

`on_idle` is the simplest entry point because it fires automatically when the Spirit has no pending work. You do not need to set up IAC frame routing, scheduled invocations, or any external triggers — the kernel fires `on_idle` after the configurable `idle_timeout_ms` window (default 30 000 ms) elapses with no inbound frames.

This pattern is the right starting point when:

- You are learning the MAOS Spirit model for the first time.
- Your Spirit does self-contained periodic work (polling, summarisation, health checks).
- You want a compiling baseline before adding more hooks.

Every hook receives `&mut Ctx`, which carries the cancellation signal, capability handle, and mailbox handle. Even in a minimal Spirit you should check `ctx.cancellation().is_cancelled()` before doing expensive work — see [Cancellation Handling](./cancellation-handling).
