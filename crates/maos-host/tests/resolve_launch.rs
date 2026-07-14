//! Unit tests for `SpiritHostPort::resolve_launch` via a test-double adapter.
//!
//! Story 11.1a Task 1: verify `WasmComponent` → runner program + prepended
//! component arg; `NativeSubprocess` → identity.

use maos_host::{
    SpiritForm, SpiritHostError, SpiritHostPort, SpiritLaunchPlan, SpiritLaunchRequest, WireShape,
};

/// Test-double adapter that resolves WASM forms without real wasmtime.
struct TestHostAdapter {
    runner_program: String,
    default_fuel: u64,
}

impl SpiritHostPort for TestHostAdapter {
    fn resolve_launch(
        &self,
        request: &SpiritLaunchRequest,
    ) -> Result<SpiritLaunchPlan, SpiritHostError> {
        match request.form {
            SpiritForm::NativeSubprocess => Ok(SpiritLaunchPlan {
                program: request.artifact.clone(),
                argv: vec![],
                env: vec![],
                wire: WireShape::ContentLengthCbor,
            }),
            SpiritForm::WasmComponent => {
                if request.artifact.is_empty() {
                    return Err(SpiritHostError::InvalidComponent {
                        reason: "empty artifact path".to_string(),
                    });
                }
                let fuel = request
                    .form_config
                    .iter()
                    .find(|(k, _)| k == "fuel")
                    .and_then(|(_, v)| v.parse::<u64>().ok())
                    .unwrap_or(self.default_fuel);
                Ok(SpiritLaunchPlan {
                    program: self.runner_program.clone(),
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

#[test]
fn native_subprocess_is_identity() {
    let host = TestHostAdapter {
        runner_program: "/usr/bin/maos-wasm-runner".to_string(),
        default_fuel: 1_000_000,
    };
    let request = SpiritLaunchRequest {
        form: SpiritForm::NativeSubprocess,
        artifact: "/usr/bin/my-spirit".to_string(),
        form_config: vec![],
    };
    let plan = host.resolve_launch(&request).unwrap();

    assert_eq!(plan.program, "/usr/bin/my-spirit");
    assert!(
        plan.argv.is_empty(),
        "native form should have no extra argv"
    );
    assert!(plan.env.is_empty());
    assert_eq!(plan.wire, WireShape::ContentLengthCbor);
}

#[test]
fn wasm_component_uses_runner_program() {
    let host = TestHostAdapter {
        runner_program: "/usr/bin/maos-wasm-runner".to_string(),
        default_fuel: 1_000_000,
    };
    let request = SpiritLaunchRequest {
        form: SpiritForm::WasmComponent,
        artifact: "/opt/spirits/echo.wasm".to_string(),
        form_config: vec![],
    };
    let plan = host.resolve_launch(&request).unwrap();

    assert_eq!(plan.program, "/usr/bin/maos-wasm-runner");
    assert_eq!(
        plan.argv,
        vec!["--component", "/opt/spirits/echo.wasm", "--fuel", "1000000"]
    );
    assert_eq!(plan.wire, WireShape::ContentLengthCbor);
}

#[test]
fn wasm_component_respects_fuel_config() {
    let host = TestHostAdapter {
        runner_program: "/usr/bin/maos-wasm-runner".to_string(),
        default_fuel: 1_000_000,
    };
    let request = SpiritLaunchRequest {
        form: SpiritForm::WasmComponent,
        artifact: "/opt/spirits/echo.wasm".to_string(),
        form_config: vec![("fuel".to_string(), "5000000".to_string())],
    };
    let plan = host.resolve_launch(&request).unwrap();

    assert_eq!(
        plan.argv,
        vec!["--component", "/opt/spirits/echo.wasm", "--fuel", "5000000"]
    );
}

#[test]
fn wasm_component_empty_artifact_returns_invalid() {
    let host = TestHostAdapter {
        runner_program: "/usr/bin/maos-wasm-runner".to_string(),
        default_fuel: 1_000_000,
    };
    let request = SpiritLaunchRequest {
        form: SpiritForm::WasmComponent,
        artifact: "".to_string(),
        form_config: vec![],
    };
    let err = host.resolve_launch(&request).unwrap_err();

    match err {
        SpiritHostError::InvalidComponent { reason } => {
            assert!(reason.contains("empty"), "should mention empty artifact");
        }
        other => panic!("expected InvalidComponent, got: {other}"),
    }
}

#[test]
fn supported_forms_includes_both() {
    let host = TestHostAdapter {
        runner_program: "/usr/bin/maos-wasm-runner".to_string(),
        default_fuel: 1_000_000,
    };
    let forms = host.supported_forms();
    assert!(forms.contains(&SpiritForm::NativeSubprocess));
    assert!(forms.contains(&SpiritForm::WasmComponent));
}

#[test]
fn spirit_host_port_is_object_safe() {
    // The trait must be usable as `Arc<dyn SpiritHostPort>`.
    let host = TestHostAdapter {
        runner_program: "/usr/bin/maos-wasm-runner".to_string(),
        default_fuel: 1_000_000,
    };
    let dyn_host: std::sync::Arc<dyn SpiritHostPort> = std::sync::Arc::new(host);
    let request = SpiritLaunchRequest {
        form: SpiritForm::NativeSubprocess,
        artifact: "/usr/bin/my-spirit".to_string(),
        form_config: vec![],
    };
    let plan = dyn_host.resolve_launch(&request).unwrap();
    assert_eq!(plan.program, "/usr/bin/my-spirit");
}

#[test]
fn wasm_component_invalid_fuel_uses_default() {
    let host = TestHostAdapter {
        runner_program: "/usr/bin/maos-wasm-runner".to_string(),
        default_fuel: 2_000_000,
    };
    let request = SpiritLaunchRequest {
        form: SpiritForm::WasmComponent,
        artifact: "/opt/spirits/echo.wasm".to_string(),
        form_config: vec![("fuel".to_string(), "not-a-number".to_string())],
    };
    let plan = host.resolve_launch(&request).unwrap();

    assert_eq!(
        plan.argv,
        vec!["--component", "/opt/spirits/echo.wasm", "--fuel", "2000000"]
    );
}
