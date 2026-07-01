//! Wasmtime `Store<T>` state shared by the conformance probe and the runner.
//!
//! `wasip2`-targeted components (built via `cargo component`/`wit-bindgen`
//! with the `wasm32-wasip2` target, as `guests/echo-spirit` is) import
//! `wasi:cli`/`wasi:io` interfaces even when the guest code itself never
//! calls them — the Rust std runtime startup pulls them in. `HostState` +
//! `wasmtime_wasi::p2::add_to_linker_sync` satisfy those imports with a
//! minimal (no filesystem/network access — T2 confinement, not WASI's own
//! sandboxing, is the security boundary per ADR-031 §1) WASI context.

use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

pub struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl HostState {
    /// A minimal WASI context: no preopened directories, no network, no
    /// inherited env/args. The guest gets only what the WIT world's
    /// `handle-frame`/`on-start`/`on-shutdown` exports need — nothing.
    pub fn new() -> Self {
        Self {
            ctx: WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
        }
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}
