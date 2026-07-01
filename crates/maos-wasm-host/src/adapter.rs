#![forbid(unsafe_code)]

//! `SpiritHostPort` adapter for the WASM component form.
//!
//! Topology (identical to the Loom-lite adapter): daemon composition root
//! (sync) -> this adapter (sync trait) -> a dedicated `std::thread` for the
//! CPU-blocking wasmtime parse/instantiate conformance probe
//! (`crate::conformance::probe_component`), bounded by `timeout` so a
//! pathological `.wasm` cannot hang `resolve_launch` (no async runtime
//! handle is needed for this — the probe is sync wasmtime work off the
//! caller's thread, joined via a channel with a `recv_timeout`).

use std::sync::Arc;

use maos_host::{
    SpiritForm, SpiritHostError, SpiritHostPort, SpiritLaunchPlan, SpiritLaunchRequest, WireShape,
};

use crate::config::WasmHostConfig;

/// Adapter bridging the sync `SpiritHostPort` to the wasmtime conformance probe.
pub struct WasmHostAdapter {
    config: Arc<WasmHostConfig>,
    timeout: std::time::Duration,
}

impl WasmHostAdapter {
    /// Create a new adapter with the given config and probe timeout.
    pub fn new(config: Arc<WasmHostConfig>, timeout: std::time::Duration) -> Self {
        Self { config, timeout }
    }
}

impl SpiritHostPort for WasmHostAdapter {
    fn resolve_launch(
        &self,
        request: &SpiritLaunchRequest,
    ) -> Result<SpiritLaunchPlan, SpiritHostError> {
        match request.form {
            SpiritForm::NativeSubprocess => {
                // Identity resolution — the kernel default.
                Ok(SpiritLaunchPlan {
                    program: request.artifact.clone(),
                    argv: vec![],
                    env: vec![],
                    wire: WireShape::ContentLengthCbor,
                })
            }
            SpiritForm::WasmComponent => {
                if request.artifact.is_empty() {
                    return Err(SpiritHostError::InvalidComponent {
                        reason: "empty artifact path".to_string(),
                    });
                }

                let meta = std::fs::metadata(&request.artifact).map_err(|e| {
                    SpiritHostError::InvalidComponent {
                        reason: format!("cannot read component: {e}"),
                    }
                })?;
                if !meta.is_file() {
                    return Err(SpiritHostError::InvalidComponent {
                        reason: format!(
                            "artifact '{}' is not a regular file",
                            request.artifact
                        ),
                    });
                }
                const MAX_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;
                if meta.len() > MAX_COMPONENT_BYTES {
                    return Err(SpiritHostError::InvalidComponent {
                        reason: format!(
                            "component '{}' is {} bytes, exceeds the {MAX_COMPONENT_BYTES}-byte cap",
                            request.artifact,
                            meta.len()
                        ),
                    });
                }

                // Real WIT-conformance probe: parse as a component and check
                // it exports the maos:spirit@1.0 world's three functions. The
                // heavyweight wasmtime compile + the actual call path live in
                // the runner subprocess (this check is the admission gate,
                // not the execution); both share the same conformance bar so
                // a present-but-non-conformant component is rejected HERE,
                // before a process is even spawned.
                crate::conformance::probe_component(&request.artifact, self.timeout).map_err(
                    |e| SpiritHostError::InvalidComponent {
                        reason: format!("component does not conform to maos:spirit@1.0: {e}"),
                    },
                )?;

                // Extract fuel from form_config, defaulting to config's default.
                let fuel = resolve_fuel(&request.form_config, self.config.default_fuel);

                Ok(SpiritLaunchPlan {
                    program: self.config.runner_program.to_string_lossy().to_string(),
                    argv: vec![
                        "--component".to_string(),
                        request.artifact.clone(),
                        "--fuel".to_string(),
                        fuel.to_string(),
                    ],
                    env: vec![],
                    wire: WireShape::ContentLengthCbor,
                })
            }
        }
    }

    fn supported_forms(&self) -> &[SpiritForm] {
        &[SpiritForm::NativeSubprocess, SpiritForm::WasmComponent]
    }
}

/// Extract fuel budget from form_config, falling back to default.
fn resolve_fuel(form_config: &[(String, String)], default_fuel: u64) -> u64 {
    form_config
        .iter()
        .find(|(k, _)| k == "fuel")
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .unwrap_or(default_fuel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_fuel_uses_default_when_missing() {
        let config: Vec<(String, String)> = vec![];
        assert_eq!(resolve_fuel(&config, 1_000_000), 1_000_000);
    }

    #[test]
    fn resolve_fuel_parses_config_value() {
        let config = vec![("fuel".to_string(), "5000000".to_string())];
        assert_eq!(resolve_fuel(&config, 1_000_000), 5_000_000);
    }

    #[test]
    fn resolve_fuel_ignores_invalid_value() {
        let config = vec![("fuel".to_string(), "not-a-number".to_string())];
        assert_eq!(resolve_fuel(&config, 1_000_000), 1_000_000);
    }
}
