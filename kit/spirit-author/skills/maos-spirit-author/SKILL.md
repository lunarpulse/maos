---
name: maos-spirit-author
description: Write, validate, publish, install and run a MAOS domain Spirit (a deterministic TypeScript WebAssembly component) and get its answer with maosctl ask. Use when asked to build a Spirit for MAOS.
---

<!-- Agent Skills format: https://agentskills.io/specification (no version number is published; read 2026-11-23). -->

# Write a MAOS domain Spirit

This skill is the same path as [`../../AGENTS.md`](../../AGENTS.md), in the order you run it. Read that file
for the reasons, the frame contract and the failure table; follow the steps below exactly.

A domain Spirit is a TypeScript WebAssembly component that answers one kind of request from code in its own
source. A component has no inference import: it cannot call a model, the network or a file.

## Inputs

- `NAME`: the Spirit's name, lowercase letters, digits and hyphens (not `hello-spirit` or `butler`).
- `TAG`: `v$(maos --version | awk '{print $2}')`. `KIT`: the directory that holds this skill's `AGENTS.md`.

## Steps

1. `cargo generate --git https://github.com/lunarpulse/maos --tag "$TAG" templates/spirit-ts --name "$NAME" && cd "$NAME"`.
   Only this template works; `templates/spirit-rust` is refused by a registry.
2. Edit `src/index.ts`, `tests/spirit.test.ts`, `manifest.toml`: answer the shell request shape (`@maos-shell`,
   `human-authored`, `task-assign`, success criteria `Answer the operator's shell message`) with one
   `task-complete` addressed to `frame.frameFrom` whose `resultText` is your domain answer; return `[]` for any
   other frame. Drop the template's `recall`/`fetch`/`author` calls and the `log`, `skill` and
   `swap_invariants` manifest sections. Keep `iac.send`, `forms = ["wasm-component"]`, `T2`, schema 7.
3. `npm install && npm test && npm run build && npm run componentize`.
4. `maos-spirit validate --manifest manifest.toml --artifact dist/spirit.wasm`. Do not go on until it exits 0.
5. `WORK=$(mktemp -d)`; export `HOME="$WORK/home"`, `XDG_DATA_HOME="$HOME/.local/share"`,
   `XDG_CONFIG_HOME="$HOME/.config"`; `mkdir -p "$XDG_CONFIG_HOME/maos"`;
   `cp "$KIT/operator.toml.example" "$XDG_CONFIG_HOME/maos/operator.toml"`;
   `openssl rand -hex 32 > "$WORK/publisher.key" && chmod 600 "$WORK/publisher.key"`.
6. `maos-registry-server --dev-http --bind 127.0.0.1:6789 --root "$WORK/registry" > "$WORK/registry.log" 2>&1 &`
7. `maos-spirit publish --registry-uri http://127.0.0.1:6789/mcp --tier=public_untrusted --manifest manifest.toml --artifact dist/spirit.wasm --signing-key "$WORK/publisher.key"`, then
   `maosctl spirit install "$NAME@<version>" --registry-uri http://127.0.0.1:6789/mcp`.
8. `sed "s/NAME/$NAME/" "$KIT/host.toml.example" > "$WORK/host.toml"`; `maos init --plain`.
9. `systemd-run --user --scope -p Delegate=yes -- maos serve --config "$WORK/host.toml" > "$WORK/serve.log" 2>&1 &`
   The caps in the manifest need a delegated cgroup; without it the launch is refused.
10. Poll `maosctl agents` (tab-separated: `id`, `pid`, `state`, ...) until your Spirit's state is `Running`.
11. `maosctl ask "$NAME" "<text>" --wait`: exit 0 and `state: completed` with your answer in `result`. Ask one
    input per rule and one unknown input.
12. Kill the background jobs and report: name, version, `validate` output, the `agents` line, every `ask` output.

A published `name@version` is immutable: to change code, raise `version` in `manifest.toml` and repeat steps
3-9. A failed run is reported as it is.
