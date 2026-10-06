# example-spirit-ts

A TypeScript MAOS Spirit example that builds a `wasm-component` implementing
`maos:spirit@2.0.0`.

## Requirements

Use Node `^22.20 || ^24.12 || >=25`. The component toolchain is pinned in
`package.json` and uses only public npm packages. `@bytecodealliance/preview2-shim`
is pinned explicitly because `componentize-js` 0.23.0 imports it at runtime without
declaring it; npm 11 (bundled with Node 24) does not hoist it otherwise.

## Build and componentize

```bash
npm ci
npm test
npm run build
npm run componentize
```

`npm run build` generates type-only WIT bindings under `src/bindings/` and
compiles the guest entrypoint to `dist/index.js`. `npm run componentize` turns
that JavaScript into `dist/spirit.wasm`; the component imports no
`wasi:http` interface.

## Governed launch

The manifest declares `wasm-component`, `dist/spirit.wasm`, and T2. Build both
`maos` with `--features wasm-host` and `maos-wasm-runner` from the same checkout;
install the runner next to `maos`. The default engine-off build still refuses
WASM admission.

Launch uses a 768 MiB runner process cap,
360-second startup/active-turn budget, and CPU 10%. Guest memory remains 64 MiB
and instruction fuel remains 10 million. CPU 10% requires writable delegated
cgroup-v2 CPU and memory controllers; launch fails closed without them. An
installed runner alone is not permission to bypass admission or T2.

The daemon admits one scheduler identity, launches the contained runner, and
reports Loaded only after actual Ready. Component validation and compilation
occur in that child. Mailbox exports have active-turn deadlines; healthy idle
does not retain an in-flight assignment. Outbound TaskAssign scopes are all
checked against the actual admitted sender, never guest-echoed sender claims.
Guest output is attributed as Standard / SpiritAuto in the transparency log.
Bus emission requires the manifest's `[capabilities.required.iac].send` grant
(`"spirit:peer"` here), and each delivered frame is mediated by a kernel-issued
capability token.

The 360-second startup/active-turn budget was measured on x86_64 under the
CPU 10% cap; other architectures are not claimed verified.
Current kernel scope policy compares scope kinds, not MCP server/tool selectors.

`handleFrame` echoes the inbound frame only after `onStart`. Before startup it
throws `{ tag: "fault", val: "..." }`, which is the generated guest binding's
halt shape. Guest WASI stdout/stderr (including `console.log`) is routed to the
runner's stderr and journaled as `spirit.diagnostic` rows, separate from framed
output. Diagnostics are untrusted guest text, bounded per session (256 rows /
64 KiB; overflow counted in `dropped_diagnostics`); over-long lines are
truncated with a marker.

Generated bindings, dependencies, and build output are ignored so the template
drift check reads only committed scaffold inputs.
