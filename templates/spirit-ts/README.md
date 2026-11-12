# {{crate_name}}

A TypeScript MAOS Spirit scaffold that builds a `wasm-component` implementing
`maos:spirit@2.4.0` (world `spirit-lifecycle`: the frame exports plus the optional
`lifecycle` export).

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

## Publishing

The scaffold declares `public-untrusted`; publish with `maos-spirit publish
--tier=public_untrusted --manifest manifest.toml --artifact dist/spirit.wasm
--signing-key <publisher-key-file> --registry-uri http://127.0.0.1:6789/mcp`.
The key file is a 32-byte Ed25519 seed (raw or hex) with mode `0600`.
`maosctl spirit install <name>@<version> --registry-uri <uri>` verifies signatures,
ComplianceClaim, form and operator tier policy without a running daemon.
An installed component is recognized by the shell; a published engine-off binary
reports `wasm_engine_off`. The development registry is loopback-only until
20-1b supplies mutation authentication and publisher-name pinning.

Operator installs have a separate, atomically switched `installed-index.json`
under `~/.local/share/maos/registry`; its paths are relative to `installed/`.
The server's `spirits/` tree is an inbox, never a boot inventory. Reinstall older
packages through `maosctl spirit install` to enroll them; old inbox entries are
not implicitly trusted. Every boot re-verifies the package signature and
ComplianceClaim and compares the manifest and materialized component with the
signed bytes. A modified install gets a typed `admission_refused` outcome.
`hello-spirit` and `butler` are reserved shell names and cannot be installed.

The development HTTP endpoint accepts only JSON `POST /mcp` requests with a
loopback Host and, when present, a loopback Origin. It bounds headers to 16 KiB /
64 fields, active connections to eight, and request I/O to an absolute five
seconds; shutdown cancels active sockets before draining workers. These browser
and resource guards are not mutation authentication.


## Governed launch

The manifest declares `wasm-component`, `dist/spirit.wasm`, and T2. Build both
`maos` with `--features wasm-host` and `maos-wasm-runner` from the same checkout;
install the runner next to `maos`. The default engine-off build still refuses
WASM admission.

Launch uses a 768 MiB runner process cap,
360-second startup/active-turn budget, and CPU 100% (one full CPU). Guest memory remains 64 MiB
and instruction fuel remains 10 million. The CPU cap requires writable delegated
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

The 360-second startup/active-turn budget was measured on x86_64 under a
CPU 10% cap; other architectures are not claimed verified. The shipped cap is
100% because starting a componentized-JS guest is CPU-bound: under 10% a hot
swap of this example (which restarts the guest) outlasts the 90-second door
wall for one swap attempt (hosted: Ready after ~220 s), while at 100% start
plus hot swap took 49.5 s.
Current kernel scope policy compares scope kinds, not MCP server/tool selectors.

`handleFrame` echoes the inbound frame only after `onStart`. Before startup it
throws `{ tag: "fault", val: "..." }`, which is the generated guest binding's
halt shape. Guest WASI stdout/stderr (including `console.log`) is routed to the
runner's stderr and journaled as `spirit.diagnostic` rows, separate from framed
output. Diagnostics are untrusted guest text, bounded per session (256 rows /
64 KiB; overflow counted in `dropped_diagnostics`); over-long lines are
truncated with a marker.

## Recall demo

The schema-7 manifest grants `[capabilities.required.log]` with `recall = true`
and `fetch = true`. During `handleFrame`, the guest imports
`maos:spirit/recall@2.4.0`, recalls up to 16 entries, then fetches the inbound
frame id before echoing the frame. These imports are available during
`handleFrame` and inside the lifecycle exports `idle`, `consolidate`, `telemetry`
and `schedule`; `pause`, `resume`, `snapshot`, `swap-in` and `migrate` are
import-free.

The template does not require a pre-approved skill to launch.
`skills.author` and `skills.propose` run only inside `handleFrame` and the lifecycle exports `idle`, `consolidate`, `telemetry` and `schedule`, and return
`capability-denied`, `refused(code)` or `unavailable`. The host binds identity
and telemetry; skills are never auto-admitted and no content import is offered.
The explanation declaration is journaled, not enforced.

Recall lists frames the Spirit sent or received in this daemon run, so fetching
the received `--once` operator seed succeeds. Withheld frames are omitted from
recall; `payload-available = false` means a header-only entry. A fetch returning
`consent-withheld` means the granter's consent no longer permits payload disclosure
(including header-only policy). Success, `scope-violation`, and `consent-withheld`
are normal fetch outcomes. Imported result errors carry their variant in
`.payload`; all recall errors and other fetch errors (including
`capability-denied` and `unavailable`) propagate.

## Lifecycle and hot swap

The scaffold exports `lifecycle` (`maos:spirit/lifecycle@2.4.0`), so the kernel's
budgeted triggers reach it (`idle`, `consolidate`, `telemetry`, `schedule`, `pause`,
`resume`) and `maosctl spirit upgrade <name> --to <manifest>` swaps it in place: the
pid, held capability tokens and working memory survive, and the state `snapshot`
returns reaches the successor's `swap-in` (`migrate` first under
`--policy migrator`). A refused swap leaves the running version untouched.
`telemetry` arrives only if `[lifecycle].enabled_hooks` lists `on_telemetry_event`.
To opt out, drop the `lifecycle` export and build with `--world-name spirit`: the
Spirit then keeps running but cannot be swapped in place
(`spawned_swap_unsupported`).
