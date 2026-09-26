# Q1 — componentize-js

**Verdict: GO-WITH-CONDITIONS.** The pinned `@bytecodealliance/componentize-js@0.23.0` directly produces a valid `maos:spirit@1.0.0` component and the unmodified **release** HEAD runner runs `on-start` before a real framed `handle-frame`, exits 0, and emits a decodable frame. It is not viable in the current Node-20/debug-runner exit shape: a clean Node 20 install cannot load its required optional native binding, and the debug runner's `Component::new` exceeds its fixed 10-second watchdog.

## Measured result

| Probe | Observed result |
|---|---|
| Tool resolution | `npm ls @bytecodealliance/componentize-js` reports direct **0.23.0** and jco 1.35.0's nested **0.22.0**. Q1 invoked the direct 0.23.0 CLI, never `jco componentize`. |
| npm footprint | Node-22 `npm ci --verbose` fetched only `registry.npmjs.org` URLs (including registry tarballs/advisory POST). `npm query` found no `preinstall`, `install`, or `postinstall` scripts. No post-install binary fetch was observed. |
| Node 20 | Local Node **v20.19.2** clean `npm ci` warned `@napi-rs/lzma@1.5.1` requires Node `^22.20 \|\| ^24.12 \|\| >=25`; direct `npx componentize-js` then failed `Cannot find module '@napi-rs/lzma-linux-x64-gnu'` (exit 1). |
| Node 22 / 24 | Node **v22.23.3** performed clean `npm ci` and direct pinned CLI componentization successfully. Node **v24.21.0** ran the same direct pinned CLI successfully. |
| Artifact | Node-22 component: **12,259,559 bytes**, SHA-256 `72f6f9232be94bac3076a02610fa64de64ffdcacea890078e317043ebef374ff`. Node-24: 12,259,556 bytes, SHA-256 `4ff0e4df07014e39932752587668189d61714c48cf20ec9b2fc781ad666aa93e`. Neither is committed. |
| Interface | `wasm-tools component wit` shows `maos:spirit/frames@1.0.0` plus WASI p2 imports and the three expected exports: `handle-frame`, `on-start`, `on-shutdown`; no `wasi:http` import appears after `--disable http fetch-event`. |
| StarlingMonkey | **Embedded: yes.** The 12.26 MB artifact carries `StarlingMonkey`, `SpiderMonkey`, and `ComponentizeJS` strings (including `StarlingMonkey engine initializing`); this closes the preflight R2 question. |
| Compile watchdog | Exact runner `Config` (`consume_fuel(true)`, `wasm_component_model(true)`) in debug `Component::new`: **10,506.735 ms**, over 10 s. Optimized `Component::new`: **792.820 ms**; release runner EOF/start wall time: **835.051 ms**, exit 0. The debug runner rejects the same component as a compile-bomb, exit 3; the committed fixture control passes it. |
| Fuel | Release runner minimum: EOF-after-on-start **303,158**; one real frame **2,617,112**, an incremental **2,313,954**. Default 10,000,000 passes both. At insufficient fuel during instantiation the runner returns exit 3, not its documented out-of-fuel exit 4; during `handle-frame` it returns exit 4. |
| Stream / lifecycle | The final guest returns a frame only after `onStart` flips a module flag. The release pipeline decoded exactly one real frame and exited 0; `stderr` was empty. Thus guest `console.log` did not corrupt ADR-032 stdout, but it also produced no visible process stderr in this host configuration. |

### ≤20-line success excerpt

```text
$ target/release/q1-driver encode | target/release/maos-wasm-runner --component .../ts-spirit-node22.wasm | target/release/q1-driver decode
[stdout]
decoded[1]=IacFrame {
    frame_id: [ 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7 ],
    timestamp_ns: 123456789,
    logical_clock: 1,
    from: FrameAddress { spirit_id: SpiritId("q1-driver"), host_id: None, role: None },
    kind: TaskAssign,
    auto_marker: Kernel,
}
decoded-count=1
[stderr]

[exit 0]
```

## TypeScript guest ergonomics

- **WIT record → object: worked.** Generated `IacFrame` is a TypeScript interface; `spirit.ts` receives and returns it as an object and the decoded runner output preserves the real frame envelope.
- **`list<u8>` → `Uint8Array`: worked.** The generated `frameId` type is `Uint8Array`; the checked-in guest assigns it to a `Uint8Array` under `tsc --strict` and logs its length.
- **`u64` → `bigint`: worked.** The generated `timestampNs` is `bigint`; the guest's explicit `bigint` assignment passed strict compilation and ran in the release component.
- **`result<_, halt>` error → thrown value: behavior worked, declaration ergonomics are incomplete.** A temporary typed guest threw `{ tag: "fault", val: "q1-thrown-halt" }`; the release runner observed `Halt::Fault("q1-thrown-halt")` and exited 1. Generated declarations flatten the success type to `handleFrame(...): Array<IacFrame>` and do not advertise the throwable error type, so an author must know the WIT error shape separately.

## Price

- **Kernel Δ:** 0 (`git diff --numstat 92911f59 -- crates/maos-kernel-core/src` produced no rows).
- **Spike source:** 97 `.rs.txt` driver lines; 25 authored TypeScript lines; 333 generated declaration lines (455 total). The Rust source naming keeps it out of the Rust KLOC scanner as required.
- **New npm dependencies:** direct pinned `componentize-js@0.23.0`, `jco@1.35.0`, and `typescript@5.9.3`; the measured tree also has jco's nested componentize-js 0.22.0.
- **CI:** change the existing Node setup to Node 22; add one component-build step (`npm ci`, `npm run build`, direct `componentize-js`). Local Node-22 measurements: `npm ci` reported 2 s and componentization was 2,253.828 ms. Reserve one CI minute; that reserve is a budget, not a hosted-CI measurement.
- **Artifact:** 12.26 MB, over AC2's 1 MiB commit ceiling; build it in CI/ignored output only.

## Consequences (OLD → NEW)

- **17-3b AC1 / epic 17 line 98:** OLD: “componentize-js build” is unqualified. NEW: the exit job uses Node 22, runs `npm ci`, `npm run build`, then the **direct pinned 0.23.0** `componentize-js` CLI (not jco), and treats the >1 MiB component as a CI artifact. It must execute the component with release runner/daemon binaries, or separately change and prove the 10-second debug compile watchdog.
- **Exit line 2 / epic 17 line 20:** OLD: `cargo build ... && target/debug/maos run ...` with an unspecified TS build. NEW: after the Node-22 direct componentization step, build and invoke the WASM path in release profile (`cargo build --locked --release ...`; `target/release/maos run ...`) so the measured 792.820 ms `Component::new` remains below `COMPILE_TIMEOUT`; the current debug path rejects it at 10,506.735 ms.
- **R17-18 / epic 17 line 212:** OLD: rewrite orphan `forms = ["ts-inproc"]` to `wasm-component`. NEW: retain that deletion/rewrite and add the Q1 build conditions above—Node 22, a non-committed component artifact, and release execution—before exit line 2 can become green.
- **CI Node pins / `discipline.yml:1086-1088,2980-2982`:** OLD: both setup actions pin Node 20. NEW for 17-3b: change `example-spirit-ts-tests` (the job that gains componentization) to Node 22. This Q1 probe did not run `check-ko-coverage` or componentize there, so it supplies no evidence for changing that unrelated docs-only pin; leave it unchanged unless its own consumer is tested.

## Findings (all owned by 17-3b)

1. The Node-20 CI pin cannot perform a clean componentize-js 0.23.0 build on this Linux target; Node 22 is required.
2. The debug runner's 10-second compile watchdog rejects the embedded-engine component, while release succeeds. The exit must use release or this becomes a runner-design change with a new measured policy.
3. `console.log` preserves the ADR-032 stdout stream but is silent at the process boundary with the current WASI context; do not make console output a diagnostic acceptance signal.
4. The component embeds StarlingMonkey and is 12.26 MB, so it must remain CI-generated/ignored rather than be committed.

## Evidence

Key full transcripts are `evidence/q1-npm-ci-verbose.log`, `q1-npm-inspect.log`, `q1-componentize-node20.log`, `q1-node22-install.log`, `q1-componentize-final-node22.log`, `q1-componentize-final-node24.log`, `q1-component-metadata.log`, `q1-starlingmonkey-strings.log`, `q1-component-compile-time-final.log`, `q1-component-compile-time-release.log`, `q1-debug-runner-final.log`, `q1-release-runner-compile-wall.log`, `q1-fuel-bisect-final.log`, `q1-release-pipeline-final.log`, `q1-release-fixture-control.log`, and `q1-result-error-runner.log`. Every Q1 probe transcript begins with UTC, HEAD, an E15-A6 scope statement, and its command.

## Reproduce from repository root

Prerequisite: activate Node **22.23.3** (or an equivalent Node 22.20+ runtime) on `PATH` before the npm commands.

```sh
cd spikes/story-17-3a-wasm-recall/ts-guest
npm ci --verbose
npm ls @bytecodealliance/componentize-js
npm query ':attr(scripts, [postinstall])'
npm run build
mkdir -p out
npx componentize-js dist/spirit.js --wit ../../../wit --world-name spirit --disable http fetch-event -o out/ts-spirit-node22.wasm
cd ../../../..
cargo build --locked --offline --release --manifest-path spikes/story-17-3a-wasm-recall/q1-driver/Cargo.toml
cargo build --locked --release -p maos-wasm-host --bin maos-wasm-runner
wasm-tools component wit spikes/story-17-3a-wasm-recall/ts-guest/out/ts-spirit-node22.wasm
target/release/q1-driver compile-time spikes/story-17-3a-wasm-recall/ts-guest/out/ts-spirit-node22.wasm
target/release/q1-driver encode | target/release/maos-wasm-runner --component spikes/story-17-3a-wasm-recall/ts-guest/out/ts-spirit-node22.wasm | target/release/q1-driver decode
target/release/q1-driver encode | target/release/maos-wasm-runner --component tests/fixtures/wasm/echo_spirit_component.wasm | target/release/q1-driver decode
target/release/maos-wasm-runner --component spikes/story-17-3a-wasm-recall/ts-guest/out/ts-spirit-node22.wasm --fuel 303157 < /dev/null
target/release/maos-wasm-runner --component spikes/story-17-3a-wasm-recall/ts-guest/out/ts-spirit-node22.wasm --fuel 303158 < /dev/null
target/release/q1-driver encode | target/release/maos-wasm-runner --component spikes/story-17-3a-wasm-recall/ts-guest/out/ts-spirit-node22.wasm --fuel 2617111 | target/release/q1-driver decode
target/release/q1-driver encode | target/release/maos-wasm-runner --component spikes/story-17-3a-wasm-recall/ts-guest/out/ts-spirit-node22.wasm --fuel 2617112 | target/release/q1-driver decode
```

The two lower boundary commands intentionally fail (zero-frame with runner exit 3/4; one-frame with exit 4); their immediate successors pass. To reproduce the Node-20 condition, activate Node 20.19.2, remove ignored `ts-guest/node_modules`, run `npm ci`, then run the direct `npx componentize-js` command above; the recorded native-binding failure is expected.
