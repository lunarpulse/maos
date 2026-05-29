# @maos/spirit-ts

TypeScript SDK shim for MAOS Spirit authoring at v0.5 binding.

## What this is

This package provides:
- `Spirit` interface mirror (14 hooks, scaffolded for TS)
- `Ctx` mock implementation with `deprecationWarnings()` surface
- `SpiritTest` harness for in-process testing
- `expectFrame`, `expectHalt`, `assert` test helpers

## What this is NOT

**This is a TEST HARNESS, not a kernel runtime.** Production TypeScript
Spirits require either:
- Subprocess form via `CliWrapperSpirit` (Story 6.2 wire protocol)
- A future kernel-side TS runtime (post-v0.5e, not in Epic 7 scope)

## Usage

```typescript
import { Spirit, Ctx } from "@maos/spirit-ts";
import { SpiritTest, assert } from "@maos/spirit-ts/spirit_test";

export class MySpirit implements Spirit {
  onIdle(ctx: Ctx): void {
    // ...
  }
}

// Test
const harness = new SpiritTest(new MySpirit());
const report = harness.run();
assert(report.hooksFired.get("on_idle") === 1, "on_idle fired");
```

## Build

```bash
npm ci
npm run build
npm test
```
