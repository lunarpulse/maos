//! Wasmtime `Store<T>` state for the contained runner.
//!
//! `wasip2`-targeted components (built via `cargo component`/`wit-bindgen`
//! with the `wasm32-wasip2` target, as `guests/echo-spirit` is) import
//! `wasi:cli`/`wasi:io` interfaces even when the guest code itself never
//! calls them — the Rust std runtime startup pulls them in. `HostState` +
//! `wasmtime_wasi::p2::add_to_linker_sync` satisfy those imports with a
//! minimal WASI context. The runner runs inside ADR-031's T2 process boundary
//! once launched by `spawn_and_bridge`; the production launch is
//! `17-3c-wasm-spirit-on-the-bus-under-t2`. WASI is not that security boundary.

use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

pub struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    pub limits: wasmtime::StoreLimits,
}

impl HostState {
    /// A minimal WASI context: no preopened directories, no network, no
    /// inherited env/args. The guest gets only what the WIT world's
    /// `handle-frame`/`on-start`/`on-shutdown` exports need — nothing.
    pub fn new() -> Self {
        Self {
            ctx: WasiCtxBuilder::new()
                .stdout(std::io::stderr())
                .stderr(std::io::stderr())
                .build(),
            table: ResourceTable::new(),
            // `memory_size` caps each linear memory; one memory makes 64 MiB
            // the aggregate guest cap (Spirit components define one memory).
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size(64 * 1024 * 1024)
                .instances(16)
                .memories(1)
                .tables(16)
                .table_elements(65_536)
                .build(),
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
