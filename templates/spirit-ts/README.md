# {{crate_name}}

A TypeScript MAOS Spirit scaffold that builds a `wasm-component` implementing
`maos:spirit@2.0.0`.

## Requirements

Use Node `^22.20 || ^24.12 || >=25`. The component toolchain is pinned in
`package.json` and uses only public npm packages. `@bytecodealliance/preview2-shim`
is pinned explicitly because `componentize-js` 0.23.0 imports it at runtime without
declaring it; npm 11 (bundled with Node 24) does not hoist it otherwise.

## Build and componentize

```bash
npm install
npm test
npm run build
npm run componentize
```

`npm run build` generates type-only WIT bindings under `src/bindings/` and
compiles the guest entrypoint to `dist/index.js`. `npm run componentize` turns
that JavaScript into `dist/spirit.wasm`; the component imports no
`wasi:http` interface.

The manifest declares the `wasm-component` form, its artifact, and T2. Today
admission intentionally refuses it as `wasm_engine_off` in the default build
and `wasm_launch_not_built` in a `wasm-host` build. Story 17-3c supplies the
production launch path.

`handleFrame` echoes the inbound frame only after `onStart`. Before startup it
throws `{ tag: "fault", val: "..." }`, which is the generated guest binding's
halt shape. `console.log` output is discarded until the future guest
diagnostics channel exists.
