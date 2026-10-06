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

[posture]
default = "assistive"
allowed_max = "assistive"

[output_shape]
required_fields = ["response"]

[budget]
context_window_size = 4096
time_cap_seconds = 60
```

Do not add `[capabilities.required]`: the WASM world imports no MAOS capabilities. Inside `[class]`, trust tiers use hyphenated values such as `org-internal`, `public-untrusted`, and `public-vetted`.

## Discussion

The manifest validates every section before its form-specific result. Today, a published/default MAOS build refuses this form as `wasm_engine_off` until Hold 2 permits the `wasm-host` engine. A build made with `--features wasm-host` instead refuses it as `wasm_launch_not_built` until `17-3c-wasm-spirit-on-the-bus-under-t2` supplies the production launch path. These are intentional typed refusals, not an unknown class or unknown form.

The generated TypeScript guest implements the world's `onStart`, `handleFrame`, and `onShutdown` exports. To halt from guest code, throw the value `throw { tag: "fault", val: "<reason>" }`; the host maps that shape to `Halt::Fault`. `console.log` is discarded today and is not a guest diagnostics channel.

Use [Hello-World Spirit](./hello-world-spirit) for in-tree first-party Rust authoring. Its `rust-inproc` form and T0 tier are deliberately different from this third-party component route.
