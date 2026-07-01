//! Wasmtime-generated host bindings for the `maos:spirit@1.0` world.
//!
//! Real component-model call path (Story 11.1a AC3) — NOT a hand-rolled
//! echo loop. The `bindgen!` macro reads `wit/spirit.wit` at compile time
//! and generates the `Spirit`/`SpiritPre` types + `call_handle_frame`/
//! `call_on_start`/`call_on_shutdown` typed methods the runner drives.

wasmtime::component::bindgen!({
    path: "../../wit/spirit.wit",
    world: "spirit",
});
