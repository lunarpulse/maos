#![forbid(unsafe_code)]

//! `SpiritHostPort` adapter for the WASM component form.
//!
//! Resolution only selects the runner, fuel and wire shape. The component is neither
//! opened nor named: it reaches the runner as the verified read-only file handed over by
//! the kernel's dedicated WASM/T2 spawn. Parsing, compilation and lifecycle validation
//! execute in the admitted runner process.

use std::sync::Arc;

use maos_host::{
    SpiritForm, SpiritHostError, SpiritHostPort, SpiritLaunchPlan, SpiritLaunchRequest, WireShape,
};

use crate::config::WasmHostConfig;

/// Adapter resolving component launch plans without executing untrusted artifacts.
pub struct WasmHostAdapter {
    config: Arc<WasmHostConfig>,
}

impl WasmHostAdapter {
    /// Create an adapter with the daemon's runner configuration.
    pub fn new(config: Arc<WasmHostConfig>) -> Self {
        Self { config }
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
                // `request.artifact` is an acquisition origin for admission, never a runner
                // input: the component reaches the runner only as the verified open file the
                // kernel's dedicated WASM/T2 spawn hands over, which appends the private
                // `--held-component-*` flags. No path is checked or emitted here.
                let fuel = resolve_fuel(&request.form_config, self.config.default_fuel);

                Ok(SpiritLaunchPlan {
                    program: self.config.runner_program.to_string_lossy().to_string(),
                    argv: vec!["--fuel".to_string(), fuel.to_string()],
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
