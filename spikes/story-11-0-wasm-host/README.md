# Story 11.0 — WASM host de-risk spike (NO-MERGE)

**Status:** hold-window spike artifact, 2026-06-29. **Not a workspace member** — the
root `Cargo.toml` `members` list is explicit (no globs), so nothing here is compiled,
linked, or counted against the kernel baseline. This directory is design + prototype
to de-risk Story 11.1a; it is **never merged as code**. The conclusions feed the 11.1a
preflight; the production code lands in `crates/maos-wasm-host/` + `maos-domain` +
`maos-bin` at 11.1a.

## What this spike answers (the 3 ACs)

1. **Prototype `SpiritHostPort` + `maos-wasm-host`** → `spirit_host_port.rs` (the trait,
   would live in `crates/maos-domain/src/ports/`) + `wasm_host_adapter.rs` (the adapter,
   would live in `crates/maos-wasm-host/src/`) + `wit/spirit.wit` (the `maos:spirit@1.0`
   typed projection of the ADR-032 frame set) + `composition_root.rs.txt` (the wiring
   sketch at `crates/maos-bin/src/main.rs`).
2. **Validate the FLAG-Winston re-pin ceiling vs 22964** → see `../../_bmad-output/implementation-artifacts/story-11-0-wasm-host-spike.md`.
   **Result: kernel-core delta ~0.** The launcher seam is composition-root + adapter only.
   The ratified ≤ +150 LOC is *headroom*, not a target.
3. **Draft ADR-031 + ADR-002/040 supersession** → `docs/adr/ADR-031-wasm-component-model-spirit-form.md`
   (and ADR-024 for the 11.4b escape detector).

## The load-bearing finding

The kernel is **already form-agnostic**. Every launch primitive runs a bare executable path:

- `spawn_and_bridge` → `Command::new(&spec.program)` — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:461`
- `spawn_sandboxed(spec, &mut Command)` — `crates/maos-kernel-core/src/security/sandbox/mod.rs:132` (`SandboxSpec` has **no** `program`/form field)
- T3: `Command::new(&argv[0])` from `spirit_binary_path` — `security/sandbox/t3/spawn.rs:114`

A WASM Spirit is therefore just `program = <wasmtime component-runner path>` with the
`.wasm` module + fuel passed as argv, sandboxed by the unchanged T2 path, speaking the
unchanged ADR-032 wire (`read_content_length`, `runtime.rs:345`; `ciborium` already a
kernel dep). The host (WIT, instantiation, fuel) is entirely user-space.

## Design: `SpiritHostPort` = a form → launch-plan resolver

The trait keeps the kernel untouched. At the composition root the daemon asks the port to
resolve a Spirit's manifest into a concrete `{program, argv, env, wire}` plan for its
declared form; the kernel's existing `spawn_and_bridge` launches the plan with no
form-specific knowledge. For the native form, resolution is identity (`program` = the
resolved binary). For the WASM form, `program` = the component runner, `argv` = the
`.wasm` module + fuel/epoch config. Mirrors the Story-10.4a `CollectiveMemoryPort` /
ADR-041 pattern verbatim (sync trait in `maos-domain`; async/blocking work — wasmtime
component compile — offloaded in the adapter via a held `tokio::runtime::Handle` +
`block_on_or_typed` guard).

## Files

| File | Would live at (11.1a) | Purpose |
|------|-----------------------|---------|
| `spirit_host_port.rs` | `crates/maos-domain/src/ports/spirit_host.rs` | the `SpiritHostPort` trait + types (zero kernel-core lines) |
| `wasm_host_adapter.rs` | `crates/maos-wasm-host/src/adapter.rs` | the `WasmHostAdapter` skeleton + guarded sync→blocking bridge |
| `wit/spirit.wit` | `crates/maos-wasm-host/wit/spirit.wit` | `maos:spirit@1.0` WIT — typed projection of the ADR-032 frame set |
| `composition_root.rs.txt` | `crates/maos-bin/src/main.rs` (~`:1683`) | the injection-site sketch (next to the Loom-lite port) |
