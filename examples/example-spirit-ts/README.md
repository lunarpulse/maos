# example-spirit-ts

A MAOS Spirit scaffolded from `templates/spirit-ts/` (Story 7.1 v0.5 binding).

## Build your first TypeScript Spirit in 30 minutes

This template scaffolds a minimal, testable MAOS Spirit in TypeScript.
By default it contains a single `onIdle` hook.

## How to run

```bash
npm ci
npm test
```

The smoke test fires `onIdle` through the `SpiritTest` harness and asserts
the hook fired exactly once.

## v0.5 TypeScript SDK caveat

The `@maos/spirit-ts` SDK is a **test harness only** at v0.5. Production
TypeScript Spirits require either:
- Subprocess form (Story 6.2 `CliWrapperSpirit` wire protocol)
- A future kernel-side TS runtime (post-v0.5e, not in Epic 7 scope)

See `sdks/spirit-ts/README.md` for details.

## Author your first Spirit — v0.5 path

```bash
cargo generate --git https://github.com/your-org/maos templates/spirit-ts --name my-ts-spirit
```

## Status

**v0.5 binding per Story 7.1.** TypeScript template ships at v0.5.
Python deferred to v1.0; Go deferred to v1.5.
