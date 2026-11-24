# Write, publish and run a MAOS domain Spirit

You are a coding agent. This file is the whole path from nothing to a signed, installed domain Spirit
that answers `maosctl ask`. Follow it in order and run each command as written. You need no MAOS source
checkout and you must not look for one. If a step fails, read **When a step fails** at the end, fix the
cause and rerun that step. Never skip `validate`, never hand-edit the installed package, and never ask
the human anything after the first message: the run is judged on whether you finish alone.

**What you build.** A *domain Spirit*: a WebAssembly component, written in TypeScript, that answers one
kind of request deterministically from code in its own source. A component has **no inference import**: it
cannot call a model, the network or a file. Do not try to; put the domain knowledge in a rule table or
function in `src/index.ts`.

**What is already on this machine** (versions are the release's; `maos --version` prints the tag):

- `maos`, `maosctl`, `maos-spirit`, `maos-registry-server`, `maos-wasm-runner` on `PATH`
  (`maos-wasm-runner` must sit next to `maos`).
- Node `^22.20 || ^24.12 || >=25` with `npm`, and `cargo-generate`.
- A Linux host with a user systemd (`systemd-run --user`) and a writable cgroup v2 hierarchy.

Set `TAG` to the release tag and `NAME` to the Spirit's name before you start:

```sh
TAG="v$(maos --version | awk '{print $2}')"      # e.g. v1.1.0
KIT=/path/to/kit/spirit-author                  # the directory that holds this file
NAME=my-domain-spirit                            # lowercase letters, digits and hyphens, 1-128 chars
```

`hello-spirit` and `butler` are reserved names. Pick a name that says what the Spirit does.

## 1. Scaffold

```sh
cargo generate --git https://github.com/lunarpulse/maos --tag "$TAG" templates/spirit-ts --name "$NAME"
cd "$NAME"
```

`templates/spirit-ts` is the **only** template that produces a Spirit a registry accepts and `maosctl ask`
can reach. `templates/spirit-rust` builds an in-process first-party Spirit: a registry refuses that form
and `ask` cannot direct it. Do not use it here.

## 2. Write the domain logic

Edit `src/index.ts`, `tests/spirit.test.ts` and `manifest.toml`. The contract with the Host is fixed:

- `maosctl ask <name> "<text>"` reaches your guest as one frame: `autoMarker === "human-authored"`,
  `kind === "task-assign"`, `frameFrom.spiritId === "@maos-shell"` (with no `hostId` and no `role`), and
  `payload.val.successCriteria === "Answer the operator's shell message"`. The text is `payload.val.goal`.
- Answer with **one** frame: the inbound frame with `to: [frame.frameFrom]`, `kind: "task-complete"` and
  `payload: { tag: "task-complete", val: { resultText: <your answer> } }`. The Host stamps the sender and
  routes the reply; the answer must be a pure function of `goal`.
- Leave every other frame unanswered (`return []`). `handleFrame` before `onStart` throws
  `{ tag: "fault", val: "..." }` (see `halt.ts`).
- Replace the template's `echo: <goal>` reply with your domain answer. Remove what you do not use: the
  template's `recall`, `fetch` and `author` calls, and then `[capabilities.required.log]`,
  `[capabilities.required.skill]` and `[swap_invariants]` from `manifest.toml`. Keep
  `[capabilities.required.iac] send = ["spirit:peer"]`: the reply is a bus emission and needs it.
- Keep the `lifecycle` export (the build targets world `spirit-lifecycle`); a stateless Spirit leaves every
  hook a no-op and `snapshot()` empty.
- In `manifest.toml` set `[class].name` to `$NAME`, write a real `description`, set `[author].name`, and keep
  `forms = ["wasm-component"]`, `trust_tier = "public-untrusted"`, `[sandbox] tier = "T2"` and
  `manifest_schema_version = 7`. Leave `[resources]` as scaffolded: `cpu_max_pct = 100`,
  `memory_max_mb = 768`. These caps are enforced; step 9 explains what they need.
- Write vitest cases in `tests/` for every rule of your domain, one unknown input, and one non-shell frame.
  Mock nothing you do not import.

## 3. Test, build, componentize

```sh
npm install
npm test
npm run build
npm run componentize
```

`npm run componentize` writes `dist/spirit.wasm`. If `npm install` warns about the Node engine, your Node is
too old: stop and use a supported one.

## 4. Validate before you publish

```sh
maos-spirit validate --manifest manifest.toml --artifact dist/spirit.wasm
```

Exit 0 means the package would pass the install gate and the Host's boot gate and that the artifact is a
component the runner can bind. Exit 1 prints one line per failed check: `[check] code: detail`. Exit 2 means
an input file could not be read. Add `--json` for a machine-readable report. **Do not continue until it
exits 0.** `validate` needs no registry, no daemon and no signing key.

## 5. Prepare a disposable home and a signing key

Use a throwaway home so nothing touches the real one, and keep the same environment for every later step
(re-export it after any shell restart):

```sh
WORK="$(mktemp -d)"
export HOME="$WORK/home" XDG_DATA_HOME="$WORK/home/.local/share" XDG_CONFIG_HOME="$WORK/home/.config"
mkdir -p "$XDG_CONFIG_HOME/maos" "$XDG_DATA_HOME"
cp "$KIT/operator.toml.example" "$XDG_CONFIG_HOME/maos/operator.toml"
openssl rand -hex 32 > "$WORK/publisher.key" && chmod 600 "$WORK/publisher.key"
```

`operator.toml` says `[registry]` / `development_http = true`, which lets the client talk to a loopback
development registry over HTTP. The key file is a 32-byte Ed25519 seed in hex, mode `0600`.

## 6. Start a loopback development registry

```sh
maos-registry-server --dev-http --bind 127.0.0.1:6789 --root "$WORK/registry" > "$WORK/registry.log" 2>&1 &
```

## 7. Publish and install

```sh
maos-spirit publish --registry-uri http://127.0.0.1:6789/mcp --tier=public_untrusted \
  --manifest manifest.toml --artifact dist/spirit.wasm --signing-key "$WORK/publisher.key"
maosctl spirit install "$NAME@0.1.0" --registry-uri http://127.0.0.1:6789/mcp
```

`publish` signs and publishes in one step. `install` runs offline: it re-checks the signature, the
ComplianceClaim, the form and the operator tier policy. Use the `version` from your manifest.

## 8. Write the Host file and initialise the home

Copy `host.toml.example` from this kit to `$WORK/host.toml`, replacing `NAME` with your Spirit's name. The
file is, in full:

```toml
[host]

[[host.spirits]]
installed = "NAME"
```

```sh
sed "s/NAME/$NAME/" "$KIT/host.toml.example" > "$WORK/host.toml"
maos init --plain
```

The Host's cast is exactly the `[[host.spirits]]` list: a Spirit the file does not name does not boot.

## 9. Start the Host under a delegated cgroup

Your Spirit declares `cpu_max_pct`, so the Host must be able to enforce it: it needs a writable, delegated
cgroup v2. Launch it so:

```sh
systemd-run --user --scope -p Delegate=yes -- maos serve --config "$WORK/host.toml" > "$WORK/serve.log" 2>&1 &
```

Without delegation the Spirit is refused at launch and never reaches `Running` (the log says why). Do not
"fix" that by removing the caps from the manifest and republishing unless the human told you to.

## 10. Wait for `Running`, then ask

`maosctl agents` prints one tab-separated line per Spirit: `id`, `pid`, `state`, and more. Wait until yours
reads `Running` (a componentized-JavaScript start can take a minute or two):

```sh
for i in $(seq 1 180); do
  maosctl agents | awk -F'\t' -v n="$NAME" '$1 == n && $3 == "Running" { found = 1 } END { exit !found }' && break
  sleep 2
done
maosctl agents
```

Then ask it something your domain answers, and read the answer:

```sh
maosctl ask "$NAME" "<a request your rules cover>" --wait
```

`--wait` prints the task object `{task_id, spirit_id, origin, state, result, ...}` and exits 0 only when
`state` is `completed`. Check that `result` is the answer your code gives for that text. Ask one input per
rule and one unknown input.

## 11. Finish

Stop the Host and the registry (`kill %1 %2`, or the job ids of steps 6 and 9). End with a short report: the
Spirit's name and version, the output of `validate`, the `maosctl agents` line, and the full `maosctl ask`
output for each input you tried. Say plainly if any step did not work; a failed run is recorded as it is.

## When a step fails

| You see | It means | Do |
|---|---|---|
| `validate`: `component_not_a_component` | the artifact is a core module or not a build output | `npm run build && npm run componentize`, validate `dist/spirit.wasm` |
| `validate`: `component_missing_export` | the guest lacks `handleFrame`, `onStart`, `onShutdown` | export all three from `src/index.ts` |
| `validate`: `component_unsupported_import` | the guest imports something the runner does not provide | remove the import; a domain Spirit needs none beyond the generated world |
| `validate`: `wasm_component_requires_t2` | `[sandbox] tier` is not `T2` | set it to `T2` |
| `validate`: `manifest_section_invalid` | a manifest section is missing or malformed | the detail names the section; `[sandbox]`, `[resources]`, `[output_shape]` and `[posture]` are required |
| `validate`: `unknown_spirit_class` | `forms` is not exactly `["wasm-component"]` | fix `forms` |
| `validate`: `first_party_form_from_registry` | you used the Rust template | start again from `templates/spirit-ts` |
| `publish` exits 1 with a tier message | `--tier` differs from the manifest's `trust_tier` | use `--tier=public_untrusted` with `public-untrusted` |
| `install` refuses a name | `hello-spirit` and `butler` are reserved | rename |
| the Spirit never reaches `Running` | read `$WORK/serve.log`; a missing cgroup delegation refuses the launch | rerun step 9 exactly |
| `ask` exits 1 `spirit_not_running` | the Spirit is not `Running` yet | wait, then ask again |
| `ask` exits 75 | the wait ran out or the queue is full | ask again; `--wait=300` waits longer |
| `ask` `state` is not `completed` | the guest faulted or gave no answer | `maosctl task show <task_id>`; read `reason`, fix the guest, republish a **new version** |

A published `name@version` is immutable: to change code, raise `[class].version` in `manifest.toml`, rebuild,
validate, publish and install the new version, and restart the Host.
