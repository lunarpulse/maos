//! Configuration for the WASM host adapter.

use std::path::PathBuf;
use std::time::Duration;

/// Configuration resolved at the daemon composition root.
///
/// `runner_program` is the absolute path to the `maos-wasm-runner` binary
/// (the wasmtime component runner that IS `BridgeSpawnSpec.program`).
/// `default_fuel` is the default fuel budget for WASM components.
#[derive(Debug, Clone)]
pub struct WasmHostConfig {
    /// Absolute path to the `maos-wasm-runner` binary.
    pub runner_program: PathBuf,
    /// Default fuel budget for WASM components (overridable per-manifest).
    pub default_fuel: u64,
    /// Timeout for component validation/compilation.
    pub validation_timeout: Duration,
}

impl WasmHostConfig {
    /// Create a new config with the given runner program path.
    pub fn new(runner_program: PathBuf, default_fuel: u64) -> Self {
        Self {
            runner_program,
            default_fuel,
            validation_timeout: Duration::from_secs(5),
        }
    }
}
