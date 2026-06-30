// SPIKE — Story 11.0 (NO-MERGE). Illustrative skeleton, not compiled.
// At 11.1a this would live at `crates/maos-wasm-host/src/adapter.rs`.
// Mirrors `crates/maos-loom-lite/src/adapter.rs` (the 10.4a adapter): holds a
// `tokio::runtime::Handle`, bridges sync→blocking via a guarded `block_on`, and
// never panics into the kernel.
#![forbid(unsafe_code)]

//! `SpiritHostPort` adapter for the WASM component form.
//!
//! Topology (identical to the Loom-lite adapter):
//!   kernel composition root (sync) → this adapter (sync trait)
//!     → block_on(async component validate/compile) on the injected handle.
//!
//! The wasmtime `Engine`/`Component` compile is CPU-blocking; doing it on a
//! `spawn_blocking` thread and re-entering async via the held handle keeps the
//! kernel runtime-agnostic. The `block_on` is GUARDED: a call from within a
//! runtime worker context, or a shut-down runtime, maps to a typed
//! `SpiritHostError::Unreachable` (no panic, no hang) — exactly the
//! `block_on_or_typed` contract in `maos-loom-lite/src/adapter.rs:64`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

// At 11.1a: `use maos_domain::ports::spirit_host::{...};`
use crate::spirit_host_port::{
    SpiritForm, SpiritHostError, SpiritHostPort, SpiritLaunchPlan, SpiritLaunchRequest, WireShape,
};

/// Config resolved at the composition root: where the component runner binary
/// is, and the default fuel budget. (In production this comes from operator
/// config, not hard-coded.)
pub struct WasmHostConfig {
    /// Absolute path to the wasmtime component-runner subprocess binary
    /// (e.g. `maos-wasm-runner`, itself a normal sandboxed subprocess).
    pub runner_program: String,
    /// Default fuel budget if the manifest does not override it.
    pub default_fuel: u64,
}

/// Adapter bridging the sync `SpiritHostPort` to async/blocking wasmtime work.
pub struct WasmHostAdapter {
    config: Arc<WasmHostConfig>,
    handle: tokio::runtime::Handle,
    timeout: Duration,
    supported: Vec<SpiritForm>,
}

impl WasmHostAdapter {
    /// Construct at the daemon composition root with the runtime handle owned
    /// there (`tokio::runtime::Handle::current()`), mirroring `LoomLiteAdapter::new`.
    pub fn new(
        config: Arc<WasmHostConfig>,
        handle: tokio::runtime::Handle,
        timeout: Duration,
    ) -> Self {
        Self {
            config,
            handle,
            timeout,
            // Native is always launchable; WASM is the v2.0 addition.
            supported: vec![SpiritForm::NativeSubprocess, SpiritForm::WasmComponent],
        }
    }
}

/// Run `fut` on the injected handle, mapping any panic-prone condition to a
/// typed `Unreachable` error — verbatim port of
/// `maos-loom-lite/src/adapter.rs:64` (`block_on_or_typed`).
fn block_on_or_typed<F, T>(handle: &tokio::runtime::Handle, fut: F) -> Result<T, SpiritHostError>
where
    F: std::future::Future<Output = T>,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(SpiritHostError::Unreachable {
            reason: "wasm host adapter invoked from within a tokio runtime worker context — \
                     it must run on a spawn_blocking thread (nested-runtime panic prevented)"
                .into(),
        });
    }
    match catch_unwind(AssertUnwindSafe(|| handle.block_on(fut))) {
        Ok(v) => Ok(v),
        Err(_) => Err(SpiritHostError::Unreachable {
            reason: "wasm host runtime handle unavailable (runtime shut down?)".into(),
        }),
    }
}

impl SpiritHostPort for WasmHostAdapter {
    fn resolve_launch(
        &self,
        request: &SpiritLaunchRequest,
    ) -> Result<SpiritLaunchPlan, SpiritHostError> {
        match request.form {
            // Native form: identity resolution. The kernel already runs this.
            SpiritForm::NativeSubprocess => Ok(SpiritLaunchPlan {
                program: request.artifact.clone(),
                argv: Vec::new(),
                env: Vec::new(),
                wire: WireShape::ContentLengthCbor,
            }),

            // WASM form: validate the component against the maos:spirit@1.0 WIT
            // world (blocking wasmtime compile, offloaded + guarded), then emit
            // a plan that launches the runner as an ordinary subprocess.
            SpiritForm::WasmComponent => {
                let config = Arc::clone(&self.config);
                let artifact = request.artifact.clone();
                let timeout_ms = self.timeout.as_millis() as u64;

                // Blocking pre-flight: compile + WIT-world conformance check.
                // (Spike stub — 11.1a wires real wasmtime `Component::from_file`
                // + `Linker` world check here.)
                block_on_or_typed(&self.handle, async move {
                    validate_component(&config, &artifact).await
                })??;
                // `??`: outer ? = block_on guard (Unreachable); inner ? below.

                let fuel = resolve_fuel(&request.form_config, self.config.default_fuel);
                Ok(SpiritLaunchPlan {
                    program: self.config.runner_program.clone(),
                    argv: vec![
                        "--component".to_string(),
                        request.artifact.clone(),
                        "--fuel".to_string(),
                        fuel.to_string(),
                    ],
                    env: Vec::new(),
                    wire: WireShape::ContentLengthCbor,
                })
                .map_err(|_: SpiritHostError| SpiritHostError::Timeout { timeout_ms })
            }
        }
    }

    fn supported_forms(&self) -> &[SpiritForm] {
        &self.supported
    }
}

/// Spike stub for the 11.1a wasmtime pre-flight: compile the component and
/// assert it satisfies the `maos:spirit@1.0` WIT world. Returns a typed
/// `InvalidComponent` on mismatch (the proven-red path).
async fn validate_component(
    _config: &WasmHostConfig,
    artifact: &str,
) -> Result<(), SpiritHostError> {
    // 11.1a: wasmtime `Engine::new` + `Component::from_file(engine, artifact)`
    // + `Linker::instantiate` against the generated WIT bindings. On failure:
    //   Err(SpiritHostError::InvalidComponent { reason })
    if artifact.is_empty() {
        return Err(SpiritHostError::InvalidComponent {
            reason: "empty component artifact path".into(),
        });
    }
    Ok(())
}

fn resolve_fuel(form_config: &[(String, String)], default_fuel: u64) -> u64 {
    form_config
        .iter()
        .find(|(k, _)| k == "fuel")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(default_fuel)
}
