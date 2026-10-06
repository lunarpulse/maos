---
title: Third-Party WASM Component Spirit
sidebar_position: 2.5
description: Build a third-party TypeScript Spirit as a WASM component with the T2 sandbox boundary.
---

# Third-Party WASM Component Spirit

## Problem

You want to author a third-party Spirit in TypeScript without placing third-party code in the MAOS process. The supported form is a self-contained WASM component built from the TypeScript scaffold and declared with the T2 sandbox boundary.

## Solution

Scaffold a project from `templates/spirit-ts`, then install its public npm dependencies and build the component:

```bash
cargo generate --git https://github.com/lunarpulse/maos \
  templates/spirit-ts --name my-wasm-spirit
cd my-wasm-spirit
npm install
npm run build && npm run componentize
```

The scaffold directly pins `@bytecodealliance/componentize-js` at `0.23.0`; its supported Node range is `^22.20 || ^24.12 || >=25`. `componentize-js` writes the component to `dist/spirit.wasm`.

Declare the component with this complete schema v5 manifest:

```toml
[class]
name = "my-wasm-spirit"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 5
min_substrate_version = "0.1.0"
forms = ["wasm-component"]
trust_tier = "public-untrusted"
description = "A third-party TypeScript WASM component Spirit."
artifact = "dist/spirit.wasm"

[author]
name = "you"

[sandbox]
tier = "T2"

[resources]
cpu_max_pct = 25
memory_max_mb = 64
fd_max = 64

[capabilities.required.iac]
send = ["spirit:peer"]

[posture]
default = "assistive"
allowed_max = "assistive"

[output_shape]
required_fields = ["response"]

[budget]
context_window_size = 4096
time_cap_seconds = 60
```

Declare only `[capabilities.required.iac]`: bus emission requires the `iac.send` grant, and the kernel mediates each delivered frame with a capability token (`"spirit:peer"` for frames addressed to Spirits, `"broadcast"` for frames with no recipient), so the Spirit's deliveries appear in `maosctl audit query --spirit`. Do not add `[capabilities.required.provider]`: the WASM world imports no provider capability. Inside `[class]`, trust tiers use hyphenated values such as `org-internal`, `public-untrusted`, and `public-vetted`.

## Discussion

The manifest validates every section before its form-specific result. A published/default MAOS build refuses this form as `wasm_engine_off` while Hold 2 keeps the `wasm-host` engine disabled. A build made with `--features wasm-host` launches the component through `maos run` inside the contained T2 runner (`maos-wasm-runner`, installed next to `maos`). In-process surfaces the contained runner does not provide — the operator door, topology, and a hot-swap successor — are refused as `spawned_surface_unsupported`. These are intentional typed refusals, not an unknown class or unknown form.

The generated TypeScript guest implements the world's `onStart`, `handleFrame`, and `onShutdown` exports. To halt from guest code, throw the value `throw { tag: "fault", val: "<reason>" }`; the host maps that shape to `Halt::Fault`. Guest WASI stdout/stderr (including `console.log`) is routed to the runner's stderr and journaled as `spirit.diagnostic` rows. Treat them as untrusted guest text: they are bounded per session (256 rows / 64 KiB, overflow counted in `dropped_diagnostics`), over-long lines are truncated with a marker, and they never become frames.

Use [Hello-World Spirit](./hello-world-spirit) for in-tree first-party Rust authoring. Its `rust-inproc` form and T0 tier are deliberately different from this third-party component route.
