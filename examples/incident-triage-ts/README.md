# incident-triage-ts

The worked **domain Spirit** of the [authoring kit](../../kit/spirit-author/AGENTS.md): a TypeScript
WebAssembly component implementing `maos:spirit@2.4.0` (world `spirit-lifecycle`) that triages an incident
description. It is built from [`templates/spirit-ts`](../../templates/spirit-ts/) and answers
`maosctl ask`.

Given text, it answers `<severity> | rule: <id> | first step: <step>` from the fixed rule table in
`src/index.ts` (first matching rule wins, case-insensitive; unknown text is
`sev4 | rule: none | first step: no rule matched`):

```text
$ maosctl ask incident-triage-ts "Customers report data loss after the migration" --wait
... "state": "completed", "result": "sev1 | rule: data-loss | first step: Freeze writes and page the data on-call." ...
```

**It is deterministic.** A WebAssembly component has no inference import, so the "domain" is this source, not
a model. The guest answers only the request shape `maosctl ask` delivers (`@maos-shell`, `human-authored`,
`task-assign`, success criteria `Answer the operator's shell message`) with one `task-complete`, and leaves any
other frame unanswered. It declares no `recall`, `fetch` or skill capability because it uses none; the manifest
keeps only `[capabilities.required.iac]`, the bus-emission grant the reply needs.

## Requirements

Node `^22.20 || ^24.12 || >=25`; the toolchain is pinned in `package.json` (see
[`example-spirit-ts`](../example-spirit-ts/README.md) for why `preview2-shim` is pinned).

## Build

```bash
npm ci
npm test
npm run build
npm run componentize
maos-spirit validate --manifest manifest.toml --artifact dist/spirit.wasm
```

## Publish, install, run and ask

From this directory, with a disposable home, exactly as the kit's steps 5 to 10 say:

```bash
WORK="$(mktemp -d)"
export HOME="$WORK/home" XDG_DATA_HOME="$WORK/home/.local/share" XDG_CONFIG_HOME="$WORK/home/.config"
mkdir -p "$XDG_CONFIG_HOME/maos" "$XDG_DATA_HOME"
cp ../../kit/spirit-author/operator.toml.example "$XDG_CONFIG_HOME/maos/operator.toml"
openssl rand -hex 32 > "$WORK/publisher.key" && chmod 600 "$WORK/publisher.key"
maos-registry-server --dev-http --bind 127.0.0.1:6789 --root "$WORK/registry" > "$WORK/registry.log" 2>&1 &
maos-spirit publish --registry-uri http://127.0.0.1:6789/mcp --tier=public_untrusted \
  --manifest manifest.toml --artifact dist/spirit.wasm --signing-key "$WORK/publisher.key"
maosctl spirit install incident-triage-ts@0.1.0 --registry-uri http://127.0.0.1:6789/mcp
sed "s/NAME/incident-triage-ts/" ../../kit/spirit-author/host.toml.example > "$WORK/host.toml"
maos init --plain
systemd-run --user --scope -p Delegate=yes -- maos serve --config "$WORK/host.toml" > "$WORK/serve.log" 2>&1 &
maosctl agents          # wait until incident-triage-ts is Running
maosctl ask incident-triage-ts "Checkout is slow, p99 latency doubled" --wait
```

## Resource caps

The manifest keeps the template's caps (`cpu_max_pct = 100`, `memory_max_mb = 768`). A declared CPU cap needs a
writable, delegated cgroup v2, and launch fails closed without one, which is why `maos serve` starts under
`systemd-run --user --scope -p Delegate=yes`. The packaged `maos.service` unit does not delegate yet
(`RELEASE-HOLDS.md`, K4), so this Spirit in a systemd-run Host needs the unit changed first.

## Tests

`npm test` runs the guest against the frame shapes above. The repository's `domain_spirit_22_7` test
(`crates/maos-bin/tests/domain_spirit_22_7.rs`) publishes **this** manifest and the component built from it,
installs it, boots `maos serve` with the kit's `host.toml.example` and asks three rule cases. The WIT in
`wit/spirit.wit` is held byte-identical to the host contract by `wit_copies_17_3b`.
