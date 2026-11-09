# example-spirit-ts

A TypeScript MAOS Spirit example that builds a `wasm-component` implementing
`maos:spirit@2.3.0`.

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

## Publish and install

The checked-in tier is `public-untrusted` (`--tier=public_untrusted` on the author
CLI). Start a development registry with
`maos-registry-server --dev-http --bind 127.0.0.1:6789 --root <throwaway-directory>`.
From this example directory, use a disposable client home rather than weakening
your production operator configuration:

```bash
export HOME="$(mktemp -d)"
export XDG_DATA_HOME="$HOME/.local/share" XDG_CONFIG_HOME="$HOME/.config"
mkdir -p "$XDG_CONFIG_HOME/maos"
printf '[registry]\ndevelopment_http = true\n' > "$XDG_CONFIG_HOME/maos/operator.toml"
maos-spirit publish --registry-uri http://127.0.0.1:6789/mcp --tier=public_untrusted \
  --manifest manifest.toml --artifact dist/spirit.wasm --signing-key <publisher-key-file>
maosctl spirit install example-spirit-ts@0.1.0 --registry-uri http://127.0.0.1:6789/mcp
```

The key file is a 32-byte Ed25519 seed (raw or hex) with mode `0600`. Installation
needs no daemon: publisher signature, signed ComplianceClaim, form admission and
operator tier policy are checked before persistence. With an operator registry
org key configured, the registry's tier signature is mandatory; pass the
corresponding seed with the server's `--signing-key`.

`maos init` followed by `maos shell` recognizes `@example-spirit-ts hello`. The
published engine-off binary reports `wasm_engine_off`, never “unknown spirit”.
The installed manifest and `dist/spirit.wasm` also support the governed `maos run
<installed-manifest-path> --once` path on an explicitly WASM-enabled build.

Operator installs have a separate, atomically switched `installed-index.json`
under `~/.local/share/maos/registry`; its paths are relative to `installed/`.
The server's `spirits/` tree is an inbox, never a boot inventory. Reinstall older
packages through `maosctl spirit install` to enroll them; old inbox entries are
not implicitly trusted. Every boot re-verifies the package signature and
ComplianceClaim and compares the manifest and materialized component with the
signed bytes. A modified install gets a typed `admission_refused` outcome.
`hello-spirit` and `butler` are reserved shell names and cannot be installed.

The development HTTP endpoint accepts only JSON `POST /mcp` requests with a
loopback Host and, when present, a matching Origin. It bounds headers to 16 KiB /
64 fields, request workers to eight/two per source, TLS/first-byte reads to one
second and request reads/writes to separate five-second absolute bounds.
Metadata bodies are capped at 64 KiB and publications at 64 MiB; the SDK supplies
the publication routing hint. Shutdown cancels active sockets before joining
workers. Browser/resource guards are not mutation authentication.

This **recipe** is loopback-only development, not authenticated/confidential
first contact. The production registry supports configured URI-bound signing
roots and TLS; see [registry security](../../crates/maos-registry/SECURITY.md).
Mutation signatures, publisher-name pins and signed yanks still apply in dev
mode. No independent third-party authorship or accredited-vetter ecosystem is
claimed. Keep the disposable home for the launch steps below, then remove it
after stopping your development processes.


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

## Recall demo

The schema-7 manifest grants `[capabilities.required.log]` with `recall = true`
and `fetch = true`. During `handleFrame`, the guest imports
`maos:spirit/recall@2.3.0`, recalls up to 16 entries, then fetches the inbound
frame id before echoing the frame. These imports are available only during
`handleFrame`, not lifecycle hooks.

No pre-approved skill is required to launch the example.
The example's first `handleFrame` calls
`skills.author` to queue `example-spirit-ts.example`; an operator must approve
it. `author` and `propose` are available only in `handleFrame`, with
`capability-denied`, `refused(code)` or `unavailable` errors. The host binds
identity and telemetry; no skill-content import is provided. The explanation
declaration is journaled, not enforced.

Recall lists frames the Spirit sent or received in this daemon run, so fetching
the received `--once` operator seed succeeds. Withheld frames are omitted from
recall; `payload-available = false` means a header-only entry. A fetch returning
`consent-withheld` means the granter's consent no longer permits payload disclosure
(including header-only policy). Success, `scope-violation`, and `consent-withheld`
are normal fetch outcomes. Imported result errors carry their variant in
`.payload`; all recall errors and other fetch errors (including
`capability-denied` and `unavailable`) propagate.
