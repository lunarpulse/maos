//! Admission-time `maos:spirit@1.0` conformance probe.
//!
//! Used by `WasmHostAdapter::resolve_launch` to reject a present-but-bad
//! `.wasm` BEFORE a runner subprocess is even spawned (AC3: "a malformed /
//! non-conformant component fails closed (`InvalidComponent`)"). Shares the
//! same conformance bar as the runner's own instantiation (both go through
//! `wit_guest::Spirit::instantiate`), so a component that passes this probe
//! is guaranteed to instantiate in the runner too.

use std::time::Duration;

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

use crate::wit_guest::Spirit;

/// Parse + instantiate `path` against the `maos:spirit@1.0` world on a
/// dedicated thread bounded by `timeout`. Returns `Ok(())` if the component
/// is well-formed AND exports `handle-frame`/`on-start`/`on-shutdown`;
/// `Err` with a human-readable reason otherwise (never panics, never hangs
/// past `timeout`).
pub fn probe_component(path: &str, timeout: Duration) -> Result<(), String> {
    let path = path.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = probe_component_blocking(&path);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(format!(
            "component validation exceeded {timeout:?} (possible compile-bomb)"
        )),
    }
}

fn probe_component_blocking(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config).map_err(|e| format!("engine init: {e}"))?;

    let component =
        Component::new(&engine, &bytes).map_err(|e| format!("not a valid component: {e}"))?;

    let mut store = Store::new(&engine, crate::host_state::HostState::new());
    let mut linker = Linker::<crate::host_state::HostState>::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .map_err(|e| format!("wasi linker setup: {e}"))?;
    Spirit::instantiate(&mut store, &component, &linker)
        .map_err(|e| format!("does not implement the spirit world: {e}"))?;

    Ok(())
}
