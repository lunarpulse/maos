//! Smoke test for {{class_name}} — fires `on_idle` through the local_runner
//! and asserts the hook fired exactly once.

use {{crate_name | snake_case}}::{{class_name}};
use {{crate_name | snake_case}}::__maos_spirit_vtable_{{class_name}};
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};

#[test]
fn on_idle_fires_once() {
    let spirit = {{class_name}};
    let vtable = __maos_spirit_vtable_{{class_name}}();
    let fixture = LocalRunnerFixture {
        invoke_on_idle: true,
        ..Default::default()
    };
    let report = LocalRunner::run(&spirit, vtable, &fixture);
    assert_eq!(report.hooks_fired.get("on_idle").copied().unwrap_or(0), 1);
}
